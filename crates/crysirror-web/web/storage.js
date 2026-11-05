// SPDX-License-Identifier: MPL-2.0
// Ready is reported on transaction completion, never merely request success.
let database;
function open() {
  return database ??= new Promise((resolve, reject) => {
    const request = indexedDB.open('crysirror-web-v1', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('files');
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error('Browser storage upgrade blocked by another tab'));
    request.onsuccess = () => resolve(request.result);
  });
}
export async function loadFiles(namespace) {
  const db = await open();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('files', 'readonly');
    const rows = []; let size = 0;
    const request = tx.objectStore('files').openCursor(IDBKeyRange.bound(`${namespace}/`, `${namespace}0`, false, true));
    request.onsuccess = () => {
      const cursor = request.result;
      if (!cursor) return;
      if (!(cursor.value instanceof Uint8Array)) { tx.abort(); return; }
      const value = cursor.value;
      size += value.byteLength;
      if (rows.length >= 4096 || value.byteLength > 4 * 1024 * 1024 || size > 64 * 1024 * 1024) { tx.abort(); return; }
      rows.push([cursor.key.slice(namespace.length + 1), value]); cursor.continue();
    };
    tx.oncomplete = () => resolve(rows);
    tx.onabort = tx.onerror = () => reject(tx.error ?? new Error('Browser storage read failed or exceeded its limit'));
  });
}
export async function commitFile(namespace, name, bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.byteLength > 4 * 1024 * 1024) throw new Error('Browser save exceeds 4 MiB');
  // The caller may supply a view into WebAssembly memory. Copy before the
  // first await: memory.grow can detach that view while IndexedDB is opening.
  const owned = bytes.slice();
  const db = await open();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('files', 'readwrite', { durability: 'strict' });
    tx.objectStore('files').put(owned, `${namespace}/${name}`);
    tx.oncomplete = () => resolve();
    tx.onabort = tx.onerror = () => reject(tx.error ?? new Error('Browser save transaction failed'));
  });
}
