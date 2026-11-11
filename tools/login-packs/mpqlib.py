import ctypes, os
# StormLib: set STORMLIB to the path of libstorm.so if it is not on the loader path
lib = ctypes.CDLL(os.environ.get("STORMLIB", "libstorm.so"))
lib.SFileOpenArchive.argtypes=[ctypes.c_char_p,ctypes.c_uint32,ctypes.c_uint32,ctypes.POINTER(ctypes.c_void_p)]
lib.SFileCreateArchive.argtypes=[ctypes.c_char_p,ctypes.c_uint32,ctypes.c_uint32,ctypes.POINTER(ctypes.c_void_p)]
lib.SFileAddFileEx.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_char_p,ctypes.c_uint32,ctypes.c_uint32,ctypes.c_uint32]
lib.SFileExtractFile.argtypes=[ctypes.c_void_p,ctypes.c_char_p,ctypes.c_char_p,ctypes.c_uint32]
lib.SFileHasFile.argtypes=[ctypes.c_void_p,ctypes.c_char_p]
lib.SFileCloseArchive.argtypes=[ctypes.c_void_p]
def open_(path):
    h=ctypes.c_void_p()
    if not lib.SFileOpenArchive(path.encode(),0,0x100,ctypes.byref(h)): raise OSError("open "+path)
    return h
def has(h,name): return bool(lib.SFileHasFile(h,name.encode()))
def extract(h,name,out):
    os.makedirs(os.path.dirname(out) or '.',exist_ok=True)
    return bool(lib.SFileExtractFile(h,name.encode(),out.encode(),0))
def create(path,maxfiles=4096):
    if os.path.exists(path): os.remove(path)
    h=ctypes.c_void_p()
    if not lib.SFileCreateArchive(path.encode(),0x01000000,maxfiles,ctypes.byref(h)): raise OSError("create "+path)
    return h
def add(h,src,name,compress=True):
    flags=0x80000000|(0x200 if compress else 0)
    if not lib.SFileAddFileEx(h,src.encode(),name.encode(),flags,2 if compress else 0,2 if compress else 0): raise OSError("add "+name)
def close(h): lib.SFileCloseArchive(h)
