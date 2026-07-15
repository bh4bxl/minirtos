MEMORY {
    /*
     * The RP2350 has either external or internal flash.
     *
     * 2 MiB is a safe default here, although a Pico 2 has 4 MiB.
     */
    FLASH : ORIGIN = 0x10000000, LENGTH = 2048K
    /*
     * RAM consists of 8 banks, SRAM0-SRAM7, with a striped mapping.
     * This is usually good for performance, as it distributes load on
     * those banks evenly.
     */
    RAM : ORIGIN = 0x20000000, LENGTH = 512K
    /*
     * RAM banks 8 and 9 use a direct mapping. They can be used to have
     * memory areas dedicated for some specific job, improving predictability
     * of access times.
     * Example: Separate stacks for core0 and core1.
     */
    SRAM8 : ORIGIN = 0x20080000, LENGTH = 4K
    SRAM9 : ORIGIN = 0x20081000, LENGTH = 4K
}
PROVIDE(__ram_start = ORIGIN(RAM));
PROVIDE(__ram_end = ORIGIN(RAM) + LENGTH(RAM));

PROVIDE(__heap_size = 64K);

PROVIDE(__kernel_stack_reserve = 16K);

PROVIDE(__stack_pool_size = 64K);
PROVIDE(__stack_pool_end = __ram_end - __kernel_stack_reserve);
PROVIDE(__stack_pool_start = __stack_pool_end - __stack_pool_size);

SECTIONS {
    /* ### Boot ROM info
     *
     * Goes after .vector_table, to keep it in the first 4K of flash
     * where the Boot ROM (and picotool) can find it
     */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH

} INSERT AFTER .vector_table;

/* move .text to start /after/ the boot info */
_stext = ADDR(.start_block) + SIZEOF(.start_block);

SECTIONS {
    /* ### Picotool 'Binary Info' Entries
     *
     * Picotool looks through this block (as we have pointers to it in our
     * header) to find interesting information.
     */
    .bi_entries : ALIGN(4)
    {
        /* We put this in the header */
        __bi_entries_start = .;
        /* Here are the entries */
        KEEP(*(.bi_entries));
        /* Keep this block a nice round size */
        . = ALIGN(4);
        /* We put this in the header */
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

/*
 * Reserve the heap after every normal static RAM section,
 * including defmt-rtt's .uninit buffer.
 */
SECTIONS {
    .heap (NOLOAD) :
    {
        . = ALIGN(8);
        __heap_start = .;

        . += __heap_size;

        . = ALIGN(8);
        __heap_end = .;
    } > RAM
} INSERT AFTER .uninit;

SECTIONS {
    /* ### Boot ROM extra info
     *
     * Goes after everything in our program, so it can contain a signature.
     */
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
        __flash_binary_end = .;
    } > FLASH

} INSERT AFTER .uninit;

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);

/* ---------- Sanity checks ---------- */
ASSERT(__heap_start >= __euninit, "Heap overlaps .uninit");
ASSERT(__heap_end <= __stack_pool_start, "Heap overlaps StackPool");
