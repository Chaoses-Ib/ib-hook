#pragma once
#ifndef IB_HIJACK_EXPORT
#include "../default.h"
#endif

#define IB_HIJACK_DLL "netutils.dll"
#define IB_HIJACK_DLL_NOEXT "netutils"
#define IB_HIJACK_DLL_DIR "C:\\Windows\\System32"
#define IB_HIJACK_DLL_DESC "Net Win32 API Helpers DLL"

IB_HIJACK_EXPORT(1, NetApiBufferAllocate)
IB_HIJACK_EXPORT(2, NetApiBufferFree)
IB_HIJACK_EXPORT(3, NetApiBufferReallocate)
IB_HIJACK_EXPORT(4, NetApiBufferSize)
IB_HIJACK_EXPORT(5, NetRemoteComputerSupports)
IB_HIJACK_EXPORT(6, NetapipBufferAllocate)
IB_HIJACK_EXPORT(7, NetpIsComputerNameValid)
IB_HIJACK_EXPORT(8, NetpIsDomainNameValid)
IB_HIJACK_EXPORT(9, NetpIsGroupNameValid)
IB_HIJACK_EXPORT(10, NetpIsRemote)
IB_HIJACK_EXPORT(11, NetpIsRemoteNameValid)
IB_HIJACK_EXPORT(12, NetpIsShareNameValid)
IB_HIJACK_EXPORT(13, NetpIsUncComputerNameValid)
IB_HIJACK_EXPORT(14, NetpIsUserNameValid)
IB_HIJACK_EXPORT(15, NetpwListCanonicalize)
IB_HIJACK_EXPORT(16, NetpwListTraverse)
IB_HIJACK_EXPORT(17, NetpwNameCanonicalize)
IB_HIJACK_EXPORT(18, NetpwNameCompare)
IB_HIJACK_EXPORT(19, NetpwNameValidate)
IB_HIJACK_EXPORT(20, NetpwPathCanonicalize)
IB_HIJACK_EXPORT(21, NetpwPathCompare)
IB_HIJACK_EXPORT(22, NetpwPathType)