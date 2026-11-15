import struct
import sys


def read_uleb(data, offset):
    result = 0
    shift = 0
    while True:
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7F) << shift
        if not (byte & 0x80):
            return result, offset
        shift += 7


def exports(path):
    data = open(path, "rb").read()

    e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
    assert data[e_lfanew:e_lfanew + 4] == b"PE\0\0", "not a PE file"

    coff = e_lfanew + 4
    number_of_sections = struct.unpack_from("<H", data, coff + 2)[0]
    size_of_optional = struct.unpack_from("<H", data, coff + 16)[0]
    optional = coff + 20
    magic = struct.unpack_from("<H", data, optional)[0]
    pe32plus = magic == 0x20B

    if pe32plus:
        data_dirs = optional + 112
    else:
        data_dirs = optional + 96

    export_rva, export_size = struct.unpack_from("<II", data, data_dirs)
    if export_rva == 0:
        print("no export directory")
        return

    sections = []
    section_table = optional + size_of_optional
    for i in range(number_of_sections):
        base = section_table + i * 40
        name = data[base:base + 8].rstrip(b"\0").decode("ascii", "replace")
        vsize, vaddr, rawsize, rawptr = struct.unpack_from("<IIII", data, base + 8)
        sections.append((name, vaddr, vsize, rawptr, rawsize))

    def rva_to_offset(rva):
        for name, vaddr, vsize, rawptr, rawsize in sections:
            if vaddr <= rva < vaddr + max(vsize, rawsize):
                return rawptr + (rva - vaddr)
        return None

    exp_offset = rva_to_offset(export_rva)
    if exp_offset is None:
        print("export directory not mapped")
        return

    (_, _, _, _, name_rva, ordinal_base, num_functions, num_names,
     functions_rva, names_rva, ordinals_rva) = struct.unpack_from("<IIHHIIIIIII", data, exp_offset)

    dll_name_offset = rva_to_offset(name_rva)
    dll_name = data[dll_name_offset:data.index(b"\0", dll_name_offset)].decode("ascii", "replace")

    names_offset = rva_to_offset(names_rva)
    functions_offset = rva_to_offset(functions_rva)
    ordinals_offset = rva_to_offset(ordinals_rva)

    print(f"module: {dll_name}")
    print(f"export directory rva=0x{export_rva:x} size={export_size}")
    print(f"ordinal base: {ordinal_base}   functions: {num_functions}   names: {num_names}")
    print()

    for i in range(num_names):
        name_rva_i = struct.unpack_from("<I", data, names_offset + i * 4)[0]
        offset = rva_to_offset(name_rva_i)
        symbol = data[offset:data.index(b"\0", offset)].decode("ascii", "replace")
        ordinal_index = struct.unpack_from("<H", data, ordinals_offset + i * 2)[0]
        func_rva = struct.unpack_from("<I", data, functions_offset + ordinal_index * 4)[0]
        forwarded = export_rva <= func_rva < export_rva + export_size
        marker = " (forwarded)" if forwarded else ""
        print(f"  [{ordinal_base + ordinal_index:>4}] 0x{func_rva:08x}  {symbol}{marker}")


path = sys.argv[1] if len(sys.argv) > 1 else r"C:\Users\rrr\spotify-rs\re\target\Spotify.dll"
exports(path)
