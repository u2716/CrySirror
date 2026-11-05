// SPDX-License-Identifier: MPL-2.0
package org.crysirror.player;
import android.view.Surface;
final class Native {
    static { System.loadLibrary("crysirror_mobile"); }
    static native String initialize(android.content.Context applicationContext);
    static native String probe(String path);
    static native String start(String request, Surface surface, int width, int height);
    static native String attach(long session, Surface surface, int width, int height);
    static native String detach(long session, long surfaceGeneration);
    static native String command(long session, String request);
    static native void destroy(long session);
}
