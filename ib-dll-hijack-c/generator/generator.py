import lief
from pathlib import Path
import click

@click.command()
@click.argument('path', type=click.Path(exists=True))
def gen(path):
    binary = lief.parse(path)  # doesn't accept Path
    
    path = Path(path)
    output = path.stem + '.h'
    out = open(output, 'w', encoding='utf8')
    print(f'Writing to "{output}"...')

    out.write(f'''#pragma once
#ifndef IB_HIJACK_EXPORT
#include "../default.h"
#endif

#define IB_HIJACK_DLL "{path.name}"
#define IB_HIJACK_DLL_NOEXT "{path.stem}"
#define IB_HIJACK_DLL_DIR "{str(path.parent).replace(chr(92), chr(92)*2)}"
''')

    res = binary.resources_manager
    if res.has_version and res.version.has_string_file_info:
        desc = res.version.string_file_info.langcode_items[0].items['FileDescription'].decode('ascii')
        out.write(f'#define IB_HIJACK_DLL_DESC "{desc}"\n')
    
    out.write('\n')

    for i, f in enumerate(binary.exported_functions):
        # 'KERNEL32.VerLanguageNameW' -> 'VerLanguageNameW'
        name = f.name[f.name.find('.') + 1:]
        
        out.write(f'IB_HIJACK_EXPORT({i+1}, {name})\n')

if __name__ == '__main__':
    gen()