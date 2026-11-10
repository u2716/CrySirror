// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
package org.crysirror.player;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.os.Bundle;
import android.os.SystemClock;
import android.system.Os;
import android.system.OsConstants;
import android.view.Surface;
import android.view.SurfaceView;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.File;
import java.io.FileDescriptor;
import java.nio.file.Files;
import java.io.FileOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.function.Predicate;

/** One device-side JNI/render lifecycle check; no owner games or test libraries. */
public final class NativeHostInstrumentation extends Instrumentation {
    private long session;
    private Activity activity;
    private String backend = "vulkan";
    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        if (arguments != null) backend = arguments.getString("backend", "vulkan");
        start();
    }
    private void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
    private JSONObject response(String value) throws Exception {
        JSONObject result = new JSONObject(value);
        require(!result.has("error"), result.toString());
        return result;
    }
    private JSONObject command(JSONObject value) throws Exception {
        return response(Native.command(session, value.toString()));
    }
    private JSONObject status() throws Exception { return command(new JSONObject().put("op", "status")); }
    private JSONObject waitFor(Predicate<JSONObject> ready) throws Exception {
        long end = SystemClock.uptimeMillis() + 45000;
        while (SystemClock.uptimeMillis() < end) {
            JSONObject value = status();
            require(!"failed".equals(value.optString("phase")), value.toString());
            if (ready.test(value)) return value;
            SystemClock.sleep(10);
        }
        throw new AssertionError("Native host deadline: " + status());
    }
    private Surface surface() throws Exception {
        final SurfaceView[] view = new SurfaceView[1];
        runOnMainSync(() -> {
            view[0] = new SurfaceView(activity);
            activity.setContentView(view[0]);
        });
        long end = SystemClock.uptimeMillis() + 10000;
        while (SystemClock.uptimeMillis() < end) {
            Surface result = view[0].getHolder().getSurface();
            if (result != null && result.isValid()) return result;
            SystemClock.sleep(10);
        }
        throw new AssertionError("No Android native surface");
    }
    private File fixture() throws Exception {
        File root = new File(getTargetContext().getCacheDir(), "native-host-smoke");
        File assets = new File(root, "assets");
        require(assets.isDirectory() || assets.mkdirs(), "Cannot create fixture assets");
        ByteBuffer bmp = ByteBuffer.allocate(70).order(ByteOrder.LITTLE_ENDIAN);
        bmp.putShort((short)0x4d42).putInt(70).putInt(0).putInt(54).putInt(40)
            .putInt(2).putInt(2).putShort((short)1).putShort((short)24).putInt(0).putInt(16)
            .putInt(0).putInt(0).putInt(0).putInt(0);
        bmp.put(new byte[]{0,(byte)190,80,0,100,(byte)230,0,0,0,100,(byte)230,0,(byte)190,80,0,0});
        try (FileOutputStream out = new FileOutputStream(new File(assets, "card.bmp"))) { out.write(bmp.array()); }
        // Nonzero, very quiet PCM triggers the real asynchronous output device.
        ByteBuffer wav = ByteBuffer.allocate(9644).order(ByteOrder.LITTLE_ENDIAN);
        wav.put("RIFF".getBytes(StandardCharsets.US_ASCII)).putInt(9636)
            .put("WAVEfmt ".getBytes(StandardCharsets.US_ASCII)).putInt(16)
            .putShort((short)1).putShort((short)1).putInt(48000).putInt(96000)
            .putShort((short)2).putShort((short)16).put("data".getBytes(StandardCharsets.US_ASCII)).putInt(9600);
        for (int sample = 0; sample < 4800; sample++) {
            wav.putShort((short)(64 * Math.sin(sample * 2 * Math.PI * 440 / 48000)));
        }
        try (FileOutputStream out = new FileOutputStream(new File(assets,"tone.wav"))) { out.write(wav.array()); }
        JSONObject sprite = new JSONObject().put("op","sprite").put("id",1).put("asset","card.bmp")
            .put("rect",new JSONArray(new int[]{40,40,160,120})).put("z",1)
            .put("tint",new JSONArray(new int[]{255,255,255,255}));
        JSONObject client = new JSONObject().put("name","fixture").put("adapter","crysirror-script-v1")
            .put("game","fixture").put("program",new JSONArray().put(sprite)
                .put(new JSONObject().put("op","play").put("channel",1).put("asset","tone.wav"))
                .put(new JSONObject().put("op","wait_input")).put(new JSONObject().put("op","wait_input")));
        JSONObject project = new JSONObject().put("schema",2).put("name","Native host lifecycle")
            .put("content_revision","1").put("canvas",new JSONArray(new int[]{320,240}))
            .put("background",new JSONArray(new int[]{20,40,70,255})).put("assets","assets")
            .put("clients",new JSONArray().put(client));
        try (FileOutputStream out = new FileOutputStream(new File(root,"project.json"))) {
            out.write(project.toString().getBytes(StandardCharsets.UTF_8));
        }
        return root;
    }
    @Override public void onStart() {
        Bundle result = new Bundle();
        int resultCode = Activity.RESULT_CANCELED;
        File stderr = new File(getTargetContext().getCacheDir(), "native-host-stderr.log");
        Bundle event = new Bundle();
        event.putString("class", getClass().getName()); event.putString("test", "testNativeSurfaceLifecycle");
        event.putInt("numtests",1); event.putInt("current",1); sendStatus(1,event);
        try {
            require("vulkan".equals(backend) || "gl".equals(backend), "Unsupported test backend: " + backend);
            // Android otherwise discards Rust panic-hook stderr before status can report it.
            FileDescriptor errorFile = Os.open(stderr.getAbsolutePath(), OsConstants.O_CREAT | OsConstants.O_WRONLY | OsConstants.O_TRUNC, 0600);
            Os.dup2(errorFile, 2); Os.close(errorFile);
            Os.setenv("WGPU_BACKEND", backend, true);
            activity = startActivitySync(new Intent(getTargetContext(), MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            response(Native.initialize(getTargetContext().getApplicationContext()));
            File root = fixture();
            JSONObject request = new JSONObject().put("game",new File(root,"project.json").getAbsolutePath())
                .put("state_dir",new File(root,"state").getAbsolutePath()).put("muted",false);
            JSONObject started = response(Native.start(request.toString(), surface(), 320, 240));
            session = started.getLong("session");
            long first = started.getLong("surface_generation");
            JSONObject initial = waitFor(v -> v.optLong("frames") >= 30);
            require(initial.optBoolean("attached"), initial.toString());
            command(new JSONObject().put("op","pause").put("paused",true));
            JSONObject paused = waitFor(v -> "paused".equals(v.optString("phase")));
            SystemClock.sleep(120);
            require(status().getLong("frames") == paused.getLong("frames"), "Paused host advanced frames");
            response(Native.detach(session,first));
            waitFor(v -> !v.optBoolean("attached"));
            JSONObject attached = response(Native.attach(session,surface(),320,240));
            long next = attached.getLong("surface_generation");
            require(next > first, "Surface generation did not change");
            response(Native.detach(session,first)); // Delayed callback must not detach the new surface.
            command(new JSONObject().put("op","pause").put("paused",false));
            long before = paused.getLong("frames");
            JSONObject resumed = waitFor(v -> v.optLong("surface_generation") == next && v.optLong("frames") > before + 3);
            require(resumed.optBoolean("attached"), resumed.toString());
            Native.destroy(session);
            require(new JSONObject(Native.command(session,"{\"op\":\"status\"}")).has("error"), "Destroyed session was accepted");
            session = 0;
            result.putString("stream","Native JNI/" + backend + " lifecycle passed: " + resumed);
            result.putInt("tests",1); result.putBoolean("ok",true); sendStatus(0,event);
            resultCode = Activity.RESULT_OK;
        } catch (Throwable error) {
            String detail = android.util.Log.getStackTraceString(error);
            try { detail += "\nNative stderr:\n" + new String(Files.readAllBytes(stderr.toPath()), StandardCharsets.UTF_8); } catch (Exception ignored) {}
            event.putString("stack",detail); sendStatus(-2,event);
            result.putString("stream",event.getString("stack")); result.putBoolean("ok",false);

        } finally {
            if (session != 0) { Native.destroy(session); session = 0; }
            if (activity != null) runOnMainSync(() -> activity.finish());
        }
        finish(resultCode,result);
    }
}
