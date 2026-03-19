# IbDllHijackLib
A C library for Windows DLL hijacking.

## Usage
```cpp
// Export functions of version.dll (by export forwarding)
#include <IbDllHijack/dlls/version.h>

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
                     )
{
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH:
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}
```

You can use the [generator](generator/README.md) to generate header files for any DLL.

## Build the test
```cmd
vcpkg install boost-test
mkdir build
cd build
cmake .. -DCMAKE_TOOLCHAIN_FILE="C:\...\vcpkg\scripts\buildsystems\vcpkg.cmake"
```

## Projects using this library
* [IbDOpusExt](https://github.com/Chaoses-Ib/IbDOpusExt)
* [IbEverythingExt](https://github.com/Chaoses-Ib/IbEverythingExt)
* [IbLogiSoftExt](https://github.com/Chaoses-Ib/IbLogiSoftExt)
* [IbOneNoteExt](https://github.com/Chaoses-Ib/IbOneNoteExt)