#pragma once
#define IB_HIJACK_PRAGMA(x) _Pragma(#x)
#define IB_HIJACK_EXPORT(ordinal, name) IB_HIJACK_PRAGMA(comment(linker, "/export:" #name "=" IB_HIJACK_DLL_DIR "\\" IB_HIJACK_DLL_NOEXT "." #name ))  // ordinal is not used