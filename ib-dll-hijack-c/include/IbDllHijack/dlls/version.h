#pragma once
#ifndef IB_HIJACK_EXPORT
#include "../default.h"
#endif

#define IB_HIJACK_DLL "version.dll"
#define IB_HIJACK_DLL_NOEXT "version"
#define IB_HIJACK_DLL_DIR "C:\\Windows\\System32"
#define IB_HIJACK_DLL_DESC "Version Checking and File Installation Libraries"

IB_HIJACK_EXPORT(1, GetFileVersionInfoA)
IB_HIJACK_EXPORT(2, GetFileVersionInfoByHandle)
IB_HIJACK_EXPORT(3, GetFileVersionInfoExA)
IB_HIJACK_EXPORT(4, GetFileVersionInfoExW)
IB_HIJACK_EXPORT(5, GetFileVersionInfoSizeA)
IB_HIJACK_EXPORT(6, GetFileVersionInfoSizeExA)
IB_HIJACK_EXPORT(7, GetFileVersionInfoSizeExW)
IB_HIJACK_EXPORT(8, GetFileVersionInfoSizeW)
IB_HIJACK_EXPORT(9, GetFileVersionInfoW)
IB_HIJACK_EXPORT(10, VerFindFileA)
IB_HIJACK_EXPORT(11, VerFindFileW)
IB_HIJACK_EXPORT(12, VerInstallFileA)
IB_HIJACK_EXPORT(13, VerInstallFileW)
IB_HIJACK_EXPORT(14, VerLanguageNameA)
IB_HIJACK_EXPORT(15, VerLanguageNameW)
IB_HIJACK_EXPORT(16, VerQueryValueA)
IB_HIJACK_EXPORT(17, VerQueryValueW)