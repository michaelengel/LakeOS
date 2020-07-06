use crate::prelude::*;
use core::cell::Cell;
use core::convert::TryFrom;
use core::mem::MaybeUninit;

use crate::vspace::*;
use crate::objects::*;
use crate::console::kprintln;
use sysapi::init::InitCSpaceSlot::*;
use sysapi::init::INIT_CSPACE_SIZE;
use sysapi::vspace::Permission;

#[no_mangle]
static mut kernel_stack: [u8; 4096] = [0; 4096];

static mut KERNEL_PGD: Table = Table::zero();
static mut KERNEL_PUD: Table = Table::zero();
static mut KERNEL_PD: Table = Table::zero();

static mut INIT_CNODE: MaybeUninit<[CNodeEntry; INIT_CSPACE_SIZE]> = MaybeUninit::uninit();
static INIT_THREAD_ELF: &'static [u8] = include_bytes!("../../../build/init_thread.elf");

#[link_section=".boot.text"]
unsafe fn init_kernel_vspace() {
    KERNEL_PGD[pgd_index!(KERNEL_BASE)] = Entry::table_entry(KERNEL_PUD.paddr());
    KERNEL_PUD[pud_index!(KERNEL_BASE)] = Entry::table_entry(KERNEL_PD.paddr());
    for i in pd_index!(KERNEL_OFFSET)..pd_index!(IO_BASE) {
        KERNEL_PD[i] = Entry::block_entry(i * 0x200000,
                                          true,
                                          true,
                                          true,
                                          Shareability::InnerSharable,
                                          AccessPermission::KernelOnly,
                                          MemoryAttr::Normal);
    }
    for i in pd_index!(IO_BASE)..512 {
        KERNEL_PD[i] = Entry::block_entry(i * 0x200000,
                                          true,
                                          true,
                                          true,
                                          Shareability::InnerSharable,
                                          AccessPermission::KernelOnly,
                                          MemoryAttr::DevicenGnRnE);
    }
    KERNEL_PUD[pud_index!(KERNEL_BASE) + 1] = Entry::block_entry(0x40000000,
                                                                 true,
                                                                 true,
                                                                 true,
                                                                 Shareability::InnerSharable,
                                                                 AccessPermission::KernelOnly,
                                                                 MemoryAttr::DevicenGnRnE);
}

#[no_mangle]
#[link_section=".boot.text"]
pub unsafe fn init_cpu() {
    use crate::arch::vspace::enable_mmu;

    init_kernel_vspace();

    enable_mmu(KERNEL_PGD.paddr());
}

#[link_section=".boot.text"]
fn initialize_init_cspace(cnode: &CNodeObj, cur_free_slot: &mut usize) {
    fn init_cspace_populate_untyped(addr_start: usize, addr_end: usize,
                                        cnode: &CNodeObj,
                                        cptr: &mut usize,
                                        is_device: bool) {
        let mut cur = addr_start;

        while cur < addr_end {
            let bit_sz = cur.trailing_zeros()
                            .min(((addr_end - cur).next_power_of_two() / 2)
                                    .trailing_zeros());
            let sz = 1 << bit_sz;

            if sz > 1 << 4 {
//                kprintln!("inserting memory 0x{:x}-0x{:x} with bit size {} @cptr {} is_device {}",
//                           cur, cur + sz, bit_sz, cptr, is_device);
                assert!(cur % sz == 0);
                cnode[*cptr].set(UntypedCap::mint(cur, bit_sz as usize, is_device));
                *cptr += 1;
            } else {
//                kprintln!("skipping memory 0x{:x}-0x{:x} with bit size {}",
//                          cur, cur + sz, bit_sz);
            }
            cur += sz;
        }
    }


    let kernel_base = PHYS_BASE;
    let kernel_top = (SYMBOL!(crate::_end) - KERNEL_OFFSET).next_power_of_two();
    /* Insert Physical memory to CSpace */
    for atag in atags::Atags::get(KERNEL_OFFSET) {
//        kprintln!("atag {:x?}", atag);
        if let atags::Atag::Mem(mem) = atag {
            let mem_start = mem.start.max(0x1000) as usize; // skip first 4k for safety
            let mem_end = (mem.start + mem.size) as usize;

            if mem_start < kernel_base {
                let untyped_end = mem_end.min(kernel_top);
                init_cspace_populate_untyped(mem_start, untyped_end, cnode, cur_free_slot, false);
            }

            if mem_end > kernel_top {
                let untyped_start = mem_start.max(kernel_base);
                init_cspace_populate_untyped(untyped_start, mem_end, cnode, cur_free_slot, false);
            }
        }
    }

    /* Insert Init CSpace itself into it */
    let cnode_cap = CNodeCap::mint(cnode.as_ptr() as usize - KERNEL_OFFSET,
                                   cnode.len().trailing_zeros() as usize,
                                   64 - cnode.len().trailing_zeros() as usize,
                                   0);
    cnode[InitCSpace as usize].set(cnode_cap);

    /* Insert monitor cap for super user to control kernel */
    cnode[Monitor as usize].set(MonitorCap::mint());

    cnode[IrqController as usize].set(InterruptCap::mint());

    /* Insert Init Thread TCB */
    alloc_obj::<TcbObj>(&cnode[UntypedStart as usize], 12, &cnode[InitTCB as usize])
        .expect("Allocating Init Thread TCB failed");

    /* allocate PGD for init thread*/
    alloc_obj::<VTableObj>(&cnode[UntypedStart as usize], 12, &cnode[InitL1PageTable as usize])
        .expect("Allocating PGD for Init Thread failed");
}

fn alloc_obj<'a, T>(untyped: &'a CNodeEntry, bit_sz: usize, slot: &'a CNodeEntry)
    -> SysResult<CapRef<'a, T>> where T: KernelObject + ?Sized
{
    let untyped_cap = UntypedCap::try_from(untyped)?;
    let slots = core::slice::from_ref(slot);
    untyped_cap.retype(T::obj_type(), bit_sz, slots)?;
    CapRef::<T>::try_from(slot)
}

fn map_frame(tcb: &TcbObj, vaddr: usize, perm: Permission, cur_free_slot: &mut usize) -> usize {
    let cspace = tcb.cspace().expect("Init CSpace not installed");
    let vspace = tcb.vspace().expect("Init VSpace not installed");

    vspace.lookup_pgd_slot(vaddr).map(|slot| {
        if slot.is_invalid() {
            alloc_obj::<VTableObj>(&cspace[UntypedStart as usize], 12, &cspace[*cur_free_slot])
                .expect("Allocating PUD failed")
                .map_vtable(&vspace, vaddr, 2)
                .expect("Installing PUD failed");
            *cur_free_slot += 1;
        }
    }).expect("Looking up PGD slot failed");

    vspace.lookup_pud_slot(vaddr).map(|slot| {
        if slot.is_invalid() {
            alloc_obj::<VTableObj>(&cspace[UntypedStart as usize], 12, &cspace[*cur_free_slot])
                .expect("Allocating PD failed")
                .map_vtable(&vspace, vaddr, 3)
                .expect("Installing PD failed");
            *cur_free_slot += 1;
        }
    }).expect("Looking up PD slot failed");

    vspace.lookup_pd_slot(vaddr).map(|slot| {
        if slot.is_invalid() {
            alloc_obj::<VTableObj>(&cspace[UntypedStart as usize], 12, &cspace[*cur_free_slot])
                .expect("Allocating PT failed")
                .map_vtable(&vspace, vaddr, 4)
                .expect("Installing PT failed");
            *cur_free_slot += 1;
        }
    }).expect("Looking up PT slot failed");

    let frame_cap = alloc_obj::<RamObj>(&cspace[UntypedStart as usize], 12, &cspace[*cur_free_slot])
                            .expect("Allocating Frame failed");
    *cur_free_slot += 1;
    frame_cap.map_page(&vspace, vaddr, perm)
        .expect("Installing frame failed");
    frame_cap.vaddr()
}

#[link_section=".boot.text"]
fn load_init_thread(tcb: &mut TcbObj, elf_file: &[u8], cur_free_slot: &mut usize) {
    use elf_rs::{Elf, ProgramType};
    use sysapi::init::{INIT_STACK_TOP, INIT_STACK_PAGES};

    let cspace = tcb.cspace().expect("Init CSpace not installed");
    let pgd_cap = VTableCap::try_from(&cspace[InitL1PageTable as usize])
                    .expect("Init PGD cap not installed");
    tcb.install_vspace(pgd_cap);

    let elf = Elf::from_bytes(elf_file).expect("Unable to parse init thread's elf file!");

    if let Elf::Elf64(e) = elf {
        for ph in e.program_header_iter()
        {
            let p_flags = ph.ph.flags();
            let perm = Permission::new (
                p_flags & 0b100 == 0b100,
                p_flags & 0b010 == 0b010,
                p_flags & 0b001 == 0b001,
            );
            let p_type = ph.ph.ph_type();

            match p_type {
                ProgramType::LOAD => {
                    let sec_base = ph.ph.offset() as usize;
                    let sec_len= ph.ph.filesz() as usize;
                    let section = &elf_file[sec_base.. sec_base + sec_len];

                    let vaddr = ph.ph.vaddr() as usize;
                    let mem_len = ph.ph.memsz() as usize;
                    let memrange = vaddr..vaddr+mem_len;

                    memrange.step_by(PAGE_SIZE)
                        .map(|vaddr| {
                            map_frame(tcb, vaddr, perm, cur_free_slot);
                            RamCap::try_from(&cspace[*cur_free_slot - 1]).expect("frame not mapped")
                        })
                        .zip(section.chunks(PAGE_SIZE).chain(core::iter::repeat(&[0u8; 4096][..])))
                        .for_each(|(mut frame_cap, page)| {
                            frame_cap[..page.len()].copy_from_slice(page);
                        })
                }
                ProgramType::GNU_STACK => {
                    for i in 0..INIT_STACK_PAGES {
                        let vaddr = INIT_STACK_TOP - PAGE_SIZE * (i + 1);

                        map_frame(tcb, vaddr, perm, cur_free_slot);
                    }
                }
                ProgramType::NOTE => {}
                p_type => {
                    panic!("Unable to handle section type {:?}", p_type);
                }
            }
        }

        let start_addr = e.header().entry_point() as usize;
        tcb.tf.set_elr(start_addr);
        tcb.tf.set_sp(INIT_STACK_TOP);
        tcb.tf.set_spsr(0b1101 << 6 | 0 << 4 | 0b00 << 2 | 0b0 << 0); // set DAIF, AArch64, EL0t
    } else {
        unimplemented!("Elf32 binary is not supported!");
    }
}

#[no_mangle]
#[link_section=".boot.text"]
pub extern "C" fn kmain() -> ! {
    use crate::scheduler::SCHEDULER;

    crate::plat::uart::init_uart();

    let mut cur_free_slot = UntypedStart as usize;

    kprintln!("PRAISE THE SUN!");

    let init_cnode_obj = unsafe {
        let cnode = INIT_CNODE.get_mut();
        for slot in cnode.iter_mut() {
            *slot = Cell::new(CapRef::<NullObj>::mint());
        }
        cnode
    };

    initialize_init_cspace(init_cnode_obj, &mut cur_free_slot);
    let mut init_tcb_cap = TcbCap::try_from(&init_cnode_obj[InitTCB as usize])
                            .expect("Init TCB cap not installed");

    let init_cnode_cap = CNodeCap::try_from(&init_cnode_obj[InitCSpace as usize]).unwrap();
    init_tcb_cap.install_cspace(&init_cnode_cap).unwrap();
    load_init_thread(&mut init_tcb_cap, &INIT_THREAD_ELF, &mut cur_free_slot);

//    kprintln!("Init Thread Info: {:x?}", *init_tcb_cap);
    kprintln!("Jumping to User Space!");

    let mut timer = crate::arch::generic_timer::Timer::new();
    timer.initialize();
    timer.tick_in(crate::TICK);

    //TODO: somehow SCHEDULER not zeroed in bss. manually init it.
    SCHEDULER.get_mut_unsafe().init();
    SCHEDULER.push(&mut init_tcb_cap);
    SCHEDULER.activate()
}