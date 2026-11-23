// SPDX-License-Identifier: MPL-2.0
package org.crysirror.player;
import android.app.Activity;
import android.os.*;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.DocumentsContract;
import android.view.*;
import android.widget.*;
import org.json.*;
import java.io.*;
import java.util.UUID;
import java.util.concurrent.Executors;

public final class MainActivity extends Activity implements SurfaceHolder.Callback {
    private final Handler main = new Handler(Looper.getMainLooper());
    private final java.util.concurrent.ExecutorService io = Executors.newSingleThreadExecutor();
    private TextView status;
    private Spinner nls;
    private String game;
    private long session;
    private SurfaceHolder activeSurface;
    private static final class SurfaceBinding {long session, generation;}
    private final java.util.IdentityHashMap<SurfaceHolder,SurfaceBinding> surfaces=new java.util.IdentityHashMap<>();
    private boolean resumed;
    // Process-scoped plain data and a weak owner; no Activity/View is retained.
    private static RetainedSession retainedProcessSession;
    private static java.lang.ref.WeakReference<MainActivity> sessionOwner=new java.lang.ref.WeakReference<>(null);
    private void retainSession() {
        if(session!=0 && sessionOwner.get()==this)retainedProcessSession=new RetainedSession(session,game,selectedNls,playing);
    }
    private static final class RetainedSession {
        long id; String game, nls; boolean playing;
        RetainedSession(long id,String game,String nls,boolean playing) {this.id=id;this.game=game;this.nls=nls;this.playing=playing;}
    }
    private boolean playing;
    private boolean importing;
    private Button importButton, playButton;
    private int importFiles;
    private long importBytes;
    private long down;
    private float px, py;
    private final Runnable poll = new Runnable() { public void run() {
        if (session != 0) {
            JSONObject r = command(object("op","status"));
            String phase = r.optString("phase");
            if (phase.equals("failed")) status.setText(r.optString("message"));
            else if (phase.equals("running")) status.setText("");
            else if (phase.equals("waiting_surface")) status.setText(r.optString("message","Waiting for display…"));
            else if (phase.equals("paused")) status.setText("Paused");
            else if (phase.equals("stopped")) library();
        }
        main.postDelayed(this,250);
    }};
    static JSONObject object(Object... pairs) {
        JSONObject result = new JSONObject();
        try { for(int i=0;i<pairs.length;i+=2) result.put((String)pairs[i],pairs[i+1]); }
        catch(JSONException e) { throw new IllegalArgumentException(e); }
        return result;
    }
    static JSONObject parse(String value) {try {return new JSONObject(value);} catch(JSONException e) {return object("error",e.toString());}}
    JSONObject command(JSONObject request) {return session==0 ? new JSONObject() : parse(Native.command(session,request.toString()));}
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        game=getPreferences(0).getString("game",null);
        JSONObject init=parse(Native.initialize(getApplicationContext()));
        RetainedSession retained=(RetainedSession)getLastNonConfigurationInstance();
        if(retained==null)retained=retainedProcessSession;
        sessionOwner=new java.lang.ref.WeakReference<>(this);
        if(retained!=null && retained.playing && retained.id!=0) {
            game=retained.game;selectedNls=retained.nls;session=retained.id;showPlayer();
        } else library();
        if(init.has("error"))status.setText(init.optString("error"));main.post(poll);
    }
    LinearLayout layout() {
        LinearLayout root=new LinearLayout(this);root.setOrientation(LinearLayout.VERTICAL);root.setPadding(16,16,16,16);
        setContentView(root);return root;
    }
    Button button(LinearLayout root,String title,Runnable run) {
        Button b=new Button(this);b.setText(title);b.setOnClickListener(v->run.run());root.addView(b);return b;
    }
    void library() {
        stop();playing=false;
        LinearLayout root=layout();
        ImageView icon=new ImageView(this);icon.setImageResource(org.crysirror.player.R.mipmap.ic_launcher);root.addView(icon,new LinearLayout.LayoutParams(128,128));
        TextView title=new TextView(this);title.setText("CrySirror");title.setTextSize(30);root.addView(title);
        status=new TextView(this);status.setText("Choose a game folder to import into application storage.");root.addView(status);
        nls=new Spinner(this);root.addView(nls);
        importButton=button(root,"Import game folder…",()->startActivityForResult(new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),1));
        importButton.setEnabled(!importing);
        Button play=button(root,"Play",this::play);playButton=play;play.setEnabled(false);
        if(game!=null) probe(play,game);
    }
    void probe(Button play,String path) {
        status.setText("Detecting engine…");
        io.execute(()-> {
            JSONObject info=parse(Native.probe(path));
            main.post(()-> {
                if(playing || isFinishing()) return;
                if(info.has("error")) {status.setText(info.optString("error"));return;}
                java.util.ArrayList<String> choices=new java.util.ArrayList<>();choices.add("auto");
                JSONArray options=info.optJSONArray("nls_options");if(options!=null) for(int i=0;i<options.length();i++) choices.add(options.optString(i));
                nls.setAdapter(new ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,choices));
                status.setText(info.optString("title")+" · "+info.optString("engine"));play.setEnabled(!importing);
            });
        });
    }
    String selectedNls="auto";
    void play() {
        if(importing)return;
        selectedNls=String.valueOf(nls.getSelectedItem());showPlayer();
    }
    void showPlayer() {
        playing=true;
        LinearLayout root=layout();
        status=new TextView(this);status.setText("Loading…");root.addView(status);
        SurfaceView view=new SurfaceView(this);root.addView(view,new LinearLayout.LayoutParams(-1,0,1));activeSurface=view.getHolder();activeSurface.addCallback(this);
        view.setOnTouchListener((v,event)-> {
            if(!playing || view.getHolder()!=activeSurface)return false;
            px=event.getX();py=event.getY();
            int action=event.getActionMasked();boolean pressed=action!=MotionEvent.ACTION_UP && action!=MotionEvent.ACTION_CANCEL;
            if(pressed)down|=1L<<16;else down&=~(1L<<16);
            command(object("op","input","x",px,"y",py,"down",down));return true;
        });
        button(root,"Menu",()-> {command(object("op","input","x",px,"y",py,"down",1L<<17));command(object("op","input","x",px,"y",py,"down",0));});
        button(root,"Close game",()->command(object("op","close")));
        button(root,"Library",this::library);
    }
    @Override public void surfaceCreated(SurfaceHolder holder) {if(holder==activeSurface)surfaces.put(holder,new SurfaceBinding());}
    @Override public void surfaceChanged(SurfaceHolder holder,int format,int width,int height) {
        if(!playing || holder!=activeSurface) return;
        SurfaceBinding binding=surfaces.get(holder);
        if(binding==null) {binding=new SurfaceBinding();surfaces.put(holder,binding);}
        if(session==0) {
            String saves=new File(getFilesDir(),"saves/"+new File(game).getName()).getAbsolutePath();
            JSONObject r=parse(Native.start(object("game",game,"state_dir",saves,"nls",selectedNls).toString(),holder.getSurface(),width,height));
            session=r.optLong("session",0);binding.session=session;binding.generation=r.optLong("surface_generation",0);
            if(r.has("error"))status.setText(r.optString("error"));
            command(object("op","pause","paused",!resumed));
            retainSession();
        } else if(binding.generation==0) {
            JSONObject r=parse(Native.attach(session,holder.getSurface(),width,height));
            binding.session=session;binding.generation=r.optLong("surface_generation",0);
            if(r.has("error"))status.setText(r.optString("error"));
        } else command(object("op","resize","width",width,"height",height));
    }
    @Override public void surfaceDestroyed(SurfaceHolder holder) {
        SurfaceBinding binding=surfaces.remove(holder);
        if(holder==activeSurface)down=0;
        if(binding!=null && binding.session!=0 && binding.session==session && binding.generation!=0) {
            JSONObject r=parse(Native.detach(binding.session,binding.generation));
            if(r.has("error") && holder==activeSurface)status.setText(r.optString("error"));
        }
    }
    void stop() {
        // Clear the Java handle first: callbacks caused by removing the view
        // cannot detach/destroy a subsequently selected game.
        long old=session;session=0;activeSurface=null;down=0;
        boolean transferred=retainedProcessSession!=null && retainedProcessSession.id==old && sessionOwner.get()!=this;
        if(old!=0 && !transferred)Native.destroy(old);
        if(!transferred && retainedProcessSession!=null && retainedProcessSession.id==old)retainedProcessSession=null;
    }
    long keyMask(int key) {
        switch(key) {
        case KeyEvent.KEYCODE_ENTER:return 1;case KeyEvent.KEYCODE_SPACE:return 2;
        case KeyEvent.KEYCODE_DPAD_LEFT:return 1L<<2;case KeyEvent.KEYCODE_DPAD_RIGHT:return 1L<<3;
        case KeyEvent.KEYCODE_DPAD_UP:return 1L<<4;case KeyEvent.KEYCODE_DPAD_DOWN:return 1L<<5;
        case KeyEvent.KEYCODE_TAB:return 1L<<6;case KeyEvent.KEYCODE_SHIFT_LEFT:case KeyEvent.KEYCODE_SHIFT_RIGHT:return 1L<<11;
        case KeyEvent.KEYCODE_CTRL_LEFT:case KeyEvent.KEYCODE_CTRL_RIGHT:return 1L<<12;
        case KeyEvent.KEYCODE_ESCAPE:return 1L<<19;default:return 0;
        }
    }
    @Override public boolean onKeyDown(int key,KeyEvent event) {
        long mask=keyMask(key);if(playing && mask!=0) {down|=mask;command(object("op","input","x",px,"y",py,"down",down));return true;}return super.onKeyDown(key,event);
    }
    @Override public boolean onKeyUp(int key,KeyEvent event) {
        long mask=keyMask(key);if(playing && mask!=0) {down&=~mask;command(object("op","input","x",px,"y",py,"down",down));return true;}return super.onKeyUp(key,event);
    }
    @Override protected void onPause() {resumed=false;down=0;command(object("op","pause","paused",true));super.onPause();}
    @Override protected void onResume() {super.onResume();resumed=true;command(object("op","pause","paused",false));}
    @Override public Object onRetainNonConfigurationInstance() {
        retainSession();return new RetainedSession(session,game,selectedNls,playing);
    }
    @Override protected void onDestroy() {
        main.removeCallbacks(poll);
        for(SurfaceBinding binding:new java.util.ArrayList<>(surfaces.values())) {
            if(binding.session==session && binding.generation!=0)Native.detach(binding.session,binding.generation);
        }
        surfaces.clear();activeSurface=null;
        if(isFinishing())stop();else retainSession();
        io.shutdownNow();super.onDestroy();
    }
    @Override public void onBackPressed() {if(playing) {command(object("op","input","x",px,"y",py,"down",1L<<17));command(object("op","input","x",px,"y",py,"down",0));}else super.onBackPressed();}
    @Override protected void onActivityResult(int request,int result,Intent data) {
        super.onActivityResult(request,result,data);
        if(request!=1 || result!=RESULT_OK || data==null || data.getData()==null)return;
        Uri tree=data.getData();importing=true;playButton.setEnabled(false);importButton.setEnabled(false);status.setText("Importing game files…");
        File destination=new File(getFilesDir(),"games/"+UUID.randomUUID());
        io.execute(()-> {
            try {
                importFiles=0;importBytes=0;
                importTree(tree,DocumentsContract.getTreeDocumentId(tree),destination,0);
                game=destination.getAbsolutePath();getPreferences(0).edit().putString("game",game).apply();main.post(()-> {importing=false;if(!isDestroyed()&&!isFinishing())library();});
            } catch(Exception e) {removeImport(destination);main.post(()-> {importing=false;if(!isDestroyed()&&!isFinishing()) {library();status.setText("Import failed: "+e.getMessage());}});}
        });
    }
    // SAF documents are opaque URIs, not filesystem paths. Import through the
    // provider, keep original bytes/names and give Rust an app-owned capability root.
    void removeImport(File file) {
        File[] children=file.listFiles();if(children!=null)for(File child:children)removeImport(child);
        file.delete();
    }
    void importTree(Uri tree,String id,File destination,int depth) throws Exception {
        if(Thread.currentThread().isInterrupted())throw new IOException("Import cancelled");
        if(depth>64)throw new IOException("Folder depth limit");
        if(!destination.mkdirs() && !destination.isDirectory())throw new IOException("Create game folder");
        Uri children=DocumentsContract.buildChildDocumentsUriUsingTree(tree,id);
        try(Cursor c=getContentResolver().query(children,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null)) {
            if(c==null)throw new IOException("Read game folder");
            while(c.moveToNext()) {
                if(++importFiles>200000)throw new IOException("File count limit");
                String child=c.getString(0),name=c.getString(1),mime=c.getString(2);
                if(name==null || name.isEmpty() || name.equals(".") || name.equals("..") || name.contains("/") || name.contains("\\") || name.indexOf(0)>=0)throw new IOException("Invalid filename");
                File target=new File(destination,name);if(target.exists())throw new IOException("Duplicate filename: "+name);
                if(DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)) {importTree(tree,child,target,depth+1);continue;}
                Uri uri=DocumentsContract.buildDocumentUriUsingTree(tree,child);
                try(InputStream in=getContentResolver().openInputStream(uri);OutputStream out=new FileOutputStream(target)) {
                    if(in==null)throw new IOException("Cannot read "+name);
                    byte[] buffer=new byte[131072];int n;
                    while((n=in.read(buffer))!=-1) {if(Thread.currentThread().isInterrupted())throw new IOException("Import cancelled");importBytes+=n;if(importBytes>32L*1024*1024*1024)throw new IOException("Import exceeds 32 GiB");out.write(buffer,0,n);}
                }
            }
        }
    }
}
