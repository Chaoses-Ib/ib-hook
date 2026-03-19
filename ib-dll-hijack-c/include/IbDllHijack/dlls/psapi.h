#pragma once
#ifndef IB_HIJACK_EXPORT
#include "../default.h"
#endif

#define IB_HIJACK_DLL "psapi.dll"
#define IB_HIJACK_DLL_NOEXT "psapi"
#define IB_HIJACK_DLL_DIR "C:\\Windows\\System32"
#define IB_HIJACK_DLL_DESC "Process Status Helper"

IB_HIJACK_EXPORT(1, EmptyWorkingSet)
IB_HIJACK_EXPORT(2, EnumDeviceDrivers)
IB_HIJACK_EXPORT(3, EnumPageFilesA)
IB_HIJACK_EXPORT(4, EnumPageFilesW)
IB_HIJACK_EXPORT(5, EnumProcessModules)
IB_HIJACK_EXPORT(6, EnumProcessModulesEx)
IB_HIJACK_EXPORT(7, EnumProcesses)
IB_HIJACK_EXPORT(8, GetDeviceDriverBaseNameA)
IB_HIJACK_EXPORT(9, GetDeviceDriverBaseNameW)
IB_HIJACK_EXPORT(10, GetDeviceDriverFileNameA)
IB_HIJACK_EXPORT(11, GetDeviceDriverFileNameW)
IB_HIJACK_EXPORT(12, GetMappedFileNameA)
IB_HIJACK_EXPORT(13, GetMappedFileNameW)
IB_HIJACK_EXPORT(14, GetModuleBaseNameA)
IB_HIJACK_EXPORT(15, GetModuleBaseNameW)
IB_HIJACK_EXPORT(16, GetModuleFileNameExA)
IB_HIJACK_EXPORT(17, GetModuleFileNameExW)
IB_HIJACK_EXPORT(18, GetModuleInformation)
IB_HIJACK_EXPORT(19, GetPerformanceInfo)
IB_HIJACK_EXPORT(20, GetProcessImageFileNameA)
IB_HIJACK_EXPORT(21, GetProcessImageFileNameW)
IB_HIJACK_EXPORT(22, GetProcessMemoryInfo)
IB_HIJACK_EXPORT(23, GetWsChanges)
IB_HIJACK_EXPORT(24, GetWsChangesEx)
IB_HIJACK_EXPORT(25, InitializeProcessForWsWatch)
IB_HIJACK_EXPORT(26, QueryWorkingSet)
IB_HIJACK_EXPORT(27, QueryWorkingSetEx)
