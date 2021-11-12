# Copyright (C) 2009-2019 Richard Smith <richard@ex-parrot.com>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with elfify.  If not, see <http://www.gnu.org/licenses/>.

## ELF Header
7F 45 4C 46			# e_ident[EI_MAG] ELF's magic number

01				# e_ident[EI_CLASS] Indicating 32 bit
01				# e_ident[EI_DATA] Indicating little endianness
01				# e_ident[EI_VERSION] Indicating original elf

00				# e_ident[EI_OSABI] Set at 0 because none cares
00				# e_ident[EI_ABIVERSION] See above

00 00 00 00 00 00 00		# e_ident[EI_PAD]

02 00				# e_type	Indicating Executable
03 00				# e_machine	Indicating x86
01 00 00 00			# e_version	Indicating original elf

1D 82 04 08			# e_entry	Address of the entry point
34 00 00 00			# e_phoff	Address of program header table
54 00 00 00			# e_shoff	Address of section header table

00 00 00 00			# e_flags

34 00				# e_ehsize	Indicating our 52 Byte header

20 00				# e_phentsize	size of a program header table
01 00				# e_phnum	number of entries in program table

28 00				# e_shentsize	size of a section header table
03 00				# e_shnum	number of entries in section table

02 00				# e_shstrndx	index of the section names

## Program Header
01 00 00 00			# p_type	PT_LOAD = 1
00 00 00 00			# p_offset

00 80 04 08			# p_vaddr
00 80 04 08			# p_physaddr

22 02 00 00			# p_filesz
22 02 00 00			# p_memsz

05 00 00 00			# p_flags
00 10 00 00			# p_align

## Section Headers

# NULL Section Header
00 00 00 00			# sh_name
00 00 00 00			# sh_type	SHT_NULL = 0
00 00 00 00			# sh_flags
00 00 00 00			# sh_addr
00 00 00 00			# sh_offset
00 00 00 00			# sh_size
00 00 00 00			# sh_link
00 00 00 00			# sh_info
00 00 00 00			# sh_addralign
00 00 00 00			# sh_entsize

# .text Section Header
01 00 00 00			# sh_name
01 00 00 00			# sh_type	SHT_PROGBITS = 1
06 00 00 00			# sh_flags
E0 80 04 08			# sh_addr
E0 00 00 00			# sh_offset
42 01 00 00			# sh_size
00 00 00 00			# sh_link
00 00 00 00			# sh_info
04 00 00 00			# sh_addralign
00 00 00 00			# sh_entsize

# .shstrtab Section Header
07 00 00 00			# sh_name
03 00 00 00			# sh_type	SHT_STRTAB = 3
00 00 00 00			# sh_flags
00 00 00 00			# sh_addr
CC 00 00 00			# sh_offset
14 00 00 00			# sh_size
00 00 00 00			# sh_link
00 00 00 00			# sh_info 
01 00 00 00			# sh_addralign
00 00 00 00			# sh_entsize

#End headers

## .shstrtab section
00				# NULL 

2E 74 65 78 74 00		# ".text\0" 

2E 73 68 73 74 72 74 61 62 00	# ".shstrtab\0" 

00 00 00

## .text section
# Prepare space for the statbuf
#0xe0:
	55			# push	%ebp
	89 E5			# mov	%esp,%ebp
	8B 75 04		# mov	0x4(%ebp),%esi
	5D			# pop	%ebp
	C3			# ret	

# Write a byte from the stack
#0xe8:
	55			# push	%ebp
	89 E5			# mov	%esp,%ebp
	BA 04 00 00 00		# mov	$0x4,%edx
	8D 4D 08		# lea	0x8(%ebp),%ecx
	B8 04 00 00 00		# mov	$0x4,%eax
	CD 80			# int	$0x80
	5D			# pop	%ebp
	C3			# ret	

# Exit successfully
#0xfc:
	BB 00 00 00 00		# mov	$0x0,%ebx
	B8 01 00 00 00		# mov	$0x1,%eax
	CD 80			# int	$0x80

# Exit with error
#0x108:
	BB 01 00 00 00		# mov	$0x1,%ebx
	B8 01 00 00 00		# mov	$0x1,%eax
	CD 80			# int	$0x80

# Main body
#0x114:
	89 E5			# mov	%esp,%ebp

# Check we have 3 args
	83 7D 00 03		# cmpl	$0x3,0x0(%ebp)
	0F 85 E8 FF FF FF	# jne	0x108

# Open input file
	B8 05 00 00 00		# mov	$0x5,%eax
	8B 5D 08		# mov	0x8(%ebp),%ebx
	31 C9			# xor	%ecx,%ecx
	CD 80			# int	$0x80
	83 F8 00		# cmp	$0x0,%eax
	0F 8C D3 FF FF FF	# jl	0x108
	50			# push	%eax

# Open output file
	B8 05 00 00 00		# mov	$0x5,%eax
	8B 5D 0C		# mov	0xc(%ebp),%ebx
	B9 42 00 00 00		# mov	$0x42,%ecx
	BA EC 01 00 00		# mov	$0x1ec,%edx
	CD 80			# int	$0x80
	83 F8 00		# cmp	$0x0,%eax
	0F 8C B5 FF FF FF	# jl	0x108
	50			# push	%eax

# Call sys_newfstat on input file
	81 EC 00 01 00 00	# sub	$0x100,%esp
	89 E1			# mov	%esp,%ecx
	8B 5D FC		# mov	-0x4(%ebp),%ebx
	B8 6C 00 00 00		# mov	$0x6c,%eax
	CD 80			# int	$0x80

	E8 75 FF FF FF		# call	0xe0

# Write first 24 bytes of ELF header
	81 EE 6B 01 00 00	# sub	$0x16b,%esi
	BA 18 00 00 00		# mov	$0x18,%edx
	89 F1			# mov	%esi,%ecx
	8B 5D F8		# mov	-0x8(%ebp),%ebx
	B8 04 00 00 00		# mov	$0x4,%eax
	CD 80			# int	$0x80

# Write e_entry field of ELF header
	8B 85 0C FF FF FF	# mov	-0xf4(%ebp),%eax
	81 C0 DB 80 04 08	# add	$0x80480db,%eax
	50			# push	%eax
	E8 54 FF FF FF		# call	0xe8

	83 C4 04		# add	$0x4,%esp
	83 C6 1C		# add	$0x1c,%esi
	BA 28 00 00 00		# mov	$0x28,%edx
	89 F1			# mov	%esi,%ecx
	B8 04 00 00 00		# mov	$0x4,%eax
	CD 80			# int	$0x80

	8B 85 0C FF FF FF	# mov	-0xf4(%ebp),%eax
	81 C0 E0 00 00 00	# add	$0xe0,%eax
	50			# push	%eax
	E8 2E FF FF FF		# call	0xe8
	E8 29 FF FF FF		# call	0xe8
	83 C4 04		# add	$0x4,%esp
	83 C6 30		# add	$0x30,%esi
	BA 44 00 00 00		# mov	$0x44,%edx
	89 F1			# mov	%esi,%ecx
	B8 04 00 00 00		# mov	$0x4,%eax
	CD 80			# int	$0x80

	FF B5 0C FF FF FF	# pushl	-0xf4(%ebp)
	E8 0A FF FF FF		# call	0xe8
	83 C4 04		# add	$0x4,%esp
	83 C6 48		# add	$0x48,%esi
	BA 4C 00 00 00		# mov	$0x4c,%edx
	89 F1			# mov	%esi,%ecx
	B8 04 00 00 00		# mov	$0x4,%eax
	CD 80			# int	$0x80
	89 E1			# mov	%esp,%ecx
#0x1f4:
	BA 00 01 00 00		# mov	$0x100,%edx
	8B 5D FC		# mov	-0x4(%ebp),%ebx
	B8 03 00 00 00		# mov	$0x3,%eax
	CD 80			# int	$0x80
	83 F8 00		# cmp	$0x0,%eax
	0F 8E F0 FE FF FF	# jle	0xfc

	89 C2			# mov	%eax,%edx
	8B 5D F8		# mov	-0x8(%ebp),%ebx
	B8 04 00 00 00		# mov	$0x4,%eax
	CD 80			# int	$0x80
	E9 D7 FF FF FF		# jmp	0x1f4
# Entry point is here:
	E9 F2 FE FF FF		# jmp	0x114
