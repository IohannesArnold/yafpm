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
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

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

54 80 04 08			# e_entry	Address of the entry point
34 00 00 00			# e_phoff	Address of program header table
00 00 00 00			# e_shoff	Address of section header table

00 00 00 00			# e_flags

34 00				# e_ehsize	Indicating our 52 Byte header

20 00				# e_phentsize	size of a program header table
01 00				# e_phnum	number of entries in program table

00 00				# e_shentsize	size of a section header table 
00 00				# e_shnum	number of entries in section table

00 00				# e_shstrndx	index of the section names

## Program Header
01 00 00 00			# p_type	PT_LOAD = 1
00 00 00 00			# p_offset

00 80 04 08			# p_vaddr
00 80 04 08			# p_physaddr

64 01 00 00			# p_filesz
64 01 00 00			# p_memsz

05 00 00 00			# p_flags
00 10 00 00			# p_align

## Begin program
# Save **argv
	89 E5			# movl	%esp, %ebp

# Check we have 3 args
	83 7D 00 03		# cmpl	$3, 0(%ebp)
	BB 01 00 00 00		# movl	$1, %ebx
	0F 85 F8 00 00 00	# jne	.exit

# Open input file
	B8 05 00 00 00		# movl	$5, %eax
	8B 5D 08		# movl	8(%ebp), %ebx
	31 C9			# xorl	%ecx, %ecx
	CD 80			# int	$0x80
	83 F8 00		# cmpl	$0, %eax
	0F 8C E3 00 00 00	# jl	.exit
	50			# pushl	%eax
	                                                
# Open output file
	B8 05 00 00 00		# movl	$5, %eax
	8B 5D 0C		# movl	12(%ebp), %ebx
	B9 42 00 00 00		# movl	$0102, %ecx
	BA EC 01 00 00		# movl	$0754, %edx
	CD 80			# int	$0x80
	83 F8 00		# cmpl	$0, %eax
	0F 8C C5 00 00 00	# jl	.exit
	50			# pushl	%eax

# Prep ecx and edx
	BA 01 00 00 00		# movl	$1, %edx
	8D 4D AC		# lea	-84(%ebp), %ecx

# Main Loop
# loop:
	E8 58 00 00 00		# call	read
	83 F8 00		# cmpl	$0, %eax
	0F 8C AE 00 00 00	# jl	.exit
	89 C3			# movl	%eax, %ebx
	0F 84 A6 00 00 00	# je	.exit

	8A 45 AC		# movb	-84(%ebp), %al

# Test for whitespace
	3C 20			# cmpb	$0x20, %al
	74 E3			# je	loop
	3C 09			# cmpb	$0x09, %al
	74 DF			# je	loop
	3C 0A			# cmpb	$0x0A, %al
	74 DB			# je	loop

	E8 16 00 00 00		# call	comment
	83 F8 00		# cmpl	$0, %eax
	75 D1			# jne	.L8
	                                          
	E8 34 00 00 00		# call	octet
	83 F8 00		# cmpl	$0, %eax
	75 C7			# jne	.L8
	                                          
	BB 01 00 00 00		# movl	$1, %ebx
	EB 7C			# jmp	.exit
# End main loop

# comment:
	3C 23			# cmpb	$0x23, %al
	74 03			# je	.L10
	31 C0			# xorl	%eax, %eax
	C3			# ret

# .L10:
	E8 11 00 00 00		# call	read
	83 F8 01		# cmpl	$1, %eax
	75 6B			# jne	.exit
	83 7D AC 0A		# cmpl	$0x0A, -84(%ebp)
	75 F0			# jne	.L10
	B8 01 00 00 00		# movl	$1, %eax
	C3			# ret

# read:
	8B 5D FC		# movl	-4(%ebp), %ebx
	B8 03 00 00 00		# movl	$3, %eax
	CD 80			# int	$0x80
	C3			# ret

# octet:
	E8 2C 00 00 00		# call	xchr
	3C FF			# cmpb	$-1, %al
	74 27			# je	.L2
	                                                    
	C0 E0 04		# salb	$4, %al
	89 C7			# movl	%eax, %edi
	E8 E2 FF FF FF		# call	read
	83 F8 01		# cmpl	$1, %eax
	75 3C			# jne	.exit
	E8 14 00 00 00		# call	xchr
	                                                    
	3C FF			# cmpb	$-1, %al
	74 33			# je	.exit
	                                                    
	01 F8			# addl	%edi, %eax
	89 45 AC		# movl	%eax, -84(%ebp)
	                                                    
	8B 5D F8		# movl	-8(%ebp), %ebx
	B8 04 00 00 00		# movl	$4, %eax
	CD 80			# int	$0x80

# .L2:
	C3			# ret

# xchr:
	8A 45 AC		# movb	-84(%ebp), %al
	3C 30			# cmpb	$0x30, %al
	7C 0E			# jl	.L4
	3C 39			# cmpb	$0x39, %al
	7E 11			# jle	.L5
	3C 41			# cmpb	$0x41, %al
	7C 06			# jl	.L4
	3C 46			# cmpb	$0x46, %al
	7E 0D			# jle	.L6
	EB 00			# jmp	.L4

# .L4:
	B8 FF FF FF FF		# movl	$-1, %eax
	EB 06			# jmp	.L7

# .L5:
	2C 30			# subb	$0x30, %al
	EB 02			# jmp	.L7

# .L6:
	2C 37			# subb	$0x37, %al

# .L7:
	C3			# ret

# .exit:
	B8 01 00 00 00		# movl	$1, %eax
	CD 80			# int	$0x80
