use alloc::{vec, vec::Vec};
use core::{fmt::Debug, marker::PhantomData};

use super::{MapArea, MemFlags, PhysAddr, PhysFrame, VirtAddr, PAGE_SIZE};

pub trait PageTableLevels: Sync + Send {
    const LEVELS: usize;
}

pub struct PageTableLevels3;
pub struct PageTableLevels4;

impl PageTableLevels for PageTableLevels3 {
    const LEVELS: usize = 3;
}

impl PageTableLevels for PageTableLevels4 {
    const LEVELS: usize = 4;
}

pub trait GenericPTE: Debug + Clone + Copy + Sync + Send + Sized {
    // Create a page table entry point to a terminate page or block.
    fn new_page(paddr: PhysAddr, flags: MemFlags, is_block: bool) -> Self;
    // Create a page table entry point to a next level page table.
    fn new_table(paddr: PhysAddr) -> Self;

    /// Returns the physical address mapped by this entry.
    fn paddr(&self) -> PhysAddr;
    /// Returns the flags of this entry.
    fn flags(&self) -> MemFlags;
    /// Returns whether this entry is zero.
    fn is_unused(&self) -> bool;
    /// Returns whether this entry flag indicates present.
    fn is_present(&self) -> bool;
    /// For non-last level translation, returns whether this entry maps to a
    /// huge frame.
    fn is_block(&self) -> bool;
    /// Set this entry to zero.
    fn clear(&mut self);
}

pub struct PageTableImpl<L: PageTableLevels, PTE: GenericPTE> {
    root_paddr: PhysAddr,
    intrm_tables: Vec<PhysFrame>,
    _phantom: PhantomData<(L, PTE)>,
}

impl<L: PageTableLevels, PTE: GenericPTE> PageTableImpl<L, PTE> {
    pub fn new() -> Self {
        let root_frame = PhysFrame::alloc_zero().unwrap();
        Self {
            root_paddr: root_frame.start_paddr(),
            intrm_tables: vec![root_frame],
            _phantom: PhantomData,
        }
    }

    pub fn clone_from(&self, start: VirtAddr, end: VirtAddr) -> Self {
        let pt = Self::new();
        if !cfg!(target_arch = "aarch64") {
            // ARMv8 doesn't need to copy kernel page table entries to user page table.
            let dst_table = unsafe {
                core::slice::from_raw_parts_mut(
                    pt.root_paddr.into_kvaddr().as_mut_ptr() as *mut PTE,
                    ENTRY_COUNT,
                )
            };
            let src_table = unsafe {
                core::slice::from_raw_parts(
                    self.root_paddr.into_kvaddr().as_ptr() as *const PTE,
                    ENTRY_COUNT,
                )
            };
            let index_fn = if L::LEVELS == 3 {
                p3_index
            } else if L::LEVELS == 4 {
                p4_index
            } else {
                unreachable!()
            };
            let start_idx = index_fn(start);
            let end_idx = index_fn(VirtAddr::new(end.as_usize() - 1)) + 1;
            dst_table[start_idx..end_idx].copy_from_slice(&src_table[start_idx..end_idx]);
        }
        pt
    }

    pub fn root_paddr(&self) -> PhysAddr {
        self.root_paddr
    }

    #[allow(dead_code)]
    pub unsafe fn from_root(root_paddr: PhysAddr) -> Self {
        Self {
            root_paddr,
            intrm_tables: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: MemFlags) {
        let entry = self.get_entry_mut_or_create(vaddr).unwrap();
        if !entry.is_unused() {
            panic!("{:#x?} is mapped before mapping", vaddr);
        }
        *entry = GenericPTE::new_page(paddr.align_down(), flags, false);
    }

    pub fn unmap(&mut self, vaddr: VirtAddr) {
        let entry = self.get_entry_mut(vaddr).unwrap();
        if entry.is_unused() {
            panic!("{:#x?} is invalid before unmapping", vaddr);
        }
        entry.clear();
    }

    pub fn query(&self, vaddr: VirtAddr) -> Option<(PhysAddr, MemFlags)> {
        let entry = self.get_entry_mut(vaddr)?;
        if entry.is_unused() {
            return None;
        }
        let off = vaddr.page_offset();
        Some((PhysAddr::new(entry.paddr().as_usize() + off), entry.flags()))
    }

    pub fn map_area(&mut self, area: &mut MapArea) {
        let mut vaddr = area.start.as_usize();
        let end = vaddr + area.size;
        while vaddr < end {
            let paddr = area.map(VirtAddr::new(vaddr));
            self.map(VirtAddr::new(vaddr), paddr, area.flags);
            vaddr += PAGE_SIZE;
        }
    }

    pub fn unmap_area(&mut self, area: &mut MapArea) {
        let mut vaddr = area.start.as_usize();
        let end = vaddr + area.size;
        while vaddr < end {
            area.unmap(VirtAddr::new(vaddr));
            self.unmap(VirtAddr::new(vaddr));
            vaddr += PAGE_SIZE;
        }
    }

    #[allow(dead_code)]
    pub fn dump(&self, limit: usize) {
        use crate::sync::SpinNoIrqLock;
        static LOCK: SpinNoIrqLock<()> = SpinNoIrqLock::new(());
        let _lock = LOCK.lock();

        println!("Root: {:x?}", self.root_paddr());
        Self::walk(
            table_of(self.root_paddr()),
            0,
            0,
            limit,
            &|level: usize, idx: usize, vaddr: usize, entry: &PTE| {
                for _ in 0..level {
                    print!("  ");
                }
                println!("[{} - {:x}], 0x{:08x?}: {:x?}", level, idx, vaddr, entry);
            },
        );
    }
}

impl<L: PageTableLevels, PTE: GenericPTE> PageTableImpl<L, PTE> {
    fn alloc_intrm_table(&mut self) -> PhysAddr {
        let frame = PhysFrame::alloc_zero().unwrap();
        let paddr = frame.start_paddr();
        self.intrm_tables.push(frame);
        paddr
    }

    fn get_entry_mut(&self, vaddr: VirtAddr) -> Option<&mut PTE> {
        let p3 = if L::LEVELS == 3 {
            table_of_mut(self.root_paddr())
        } else if L::LEVELS == 4 {
            let p4 = table_of_mut(self.root_paddr());
            let p4e = &mut p4[p4_index(vaddr)];
            next_table_mut(p4e)?
        } else {
            unreachable!()
        };
        let p3e = &mut p3[p3_index(vaddr)];

        let p2 = next_table_mut(p3e)?;
        let p2e = &mut p2[p2_index(vaddr)];

        let p1 = next_table_mut(p2e)?;
        let p1e = &mut p1[p1_index(vaddr)];
        Some(p1e)
    }

    fn get_entry_mut_or_create(&mut self, vaddr: VirtAddr) -> Option<&mut PTE> {
        let p3 = if L::LEVELS == 3 {
            table_of_mut(self.root_paddr())
        } else if L::LEVELS == 4 {
            let p4 = table_of_mut(self.root_paddr());
            let p4e = &mut p4[p4_index(vaddr)];
            next_table_mut_or_create(p4e, || self.alloc_intrm_table())?
        } else {
            unreachable!()
        };
        let p3e = &mut p3[p3_index(vaddr)];

        let p2 = next_table_mut_or_create(p3e, || self.alloc_intrm_table())?;
        let p2e = &mut p2[p2_index(vaddr)];

        let p1 = next_table_mut_or_create(p2e, || self.alloc_intrm_table())?;
        let p1e = &mut p1[p1_index(vaddr)];
        Some(p1e)
    }

    fn walk(
        table: &[PTE],
        level: usize,
        start_vaddr: usize,
        limit: usize,
        func: &impl Fn(usize, usize, usize, &PTE),
    ) {
        let mut n = 0;
        for (i, entry) in table.iter().enumerate() {
            let vaddr = start_vaddr + (i << (12 + (L::LEVELS - 1 - level) * 9));
            let vaddr = VirtAddr::new_extended(vaddr).as_usize();
            if entry.is_present() {
                func(level, i, vaddr, entry);
                if level < L::LEVELS - 1 && !entry.is_block() {
                    let table_entry = next_table_mut(entry).unwrap();
                    Self::walk(table_entry, level + 1, vaddr, limit, func);
                }
                n += 1;
                if n >= limit {
                    break;
                }
            }
        }
    }
}

const ENTRY_COUNT: usize = 512;

const fn p4_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 27)) & (ENTRY_COUNT - 1)
}

const fn p3_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 18)) & (ENTRY_COUNT - 1)
}

const fn p2_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> (12 + 9)) & (ENTRY_COUNT - 1)
}

const fn p1_index(vaddr: VirtAddr) -> usize {
    (vaddr.as_usize() >> 12) & (ENTRY_COUNT - 1)
}

fn table_of<'a, E>(paddr: PhysAddr) -> &'a [E] {
    let ptr = paddr.into_kvaddr().as_ptr() as *const E;
    unsafe { core::slice::from_raw_parts(ptr, ENTRY_COUNT) }
}

fn table_of_mut<'a, E>(paddr: PhysAddr) -> &'a mut [E] {
    let ptr = paddr.into_kvaddr().as_mut_ptr() as *mut E;
    unsafe { core::slice::from_raw_parts_mut(ptr, ENTRY_COUNT) }
}

fn next_table_mut<'a, E: GenericPTE>(entry: &E) -> Option<&'a mut [E]> {
    if !entry.is_present() {
        None
    } else {
        assert!(!entry.is_block());
        Some(table_of_mut(entry.paddr()))
    }
}

fn next_table_mut_or_create<'a, E: GenericPTE>(
    entry: &mut E,
    mut allocator: impl FnMut() -> PhysAddr,
) -> Option<&'a mut [E]> {
    if entry.is_unused() {
        let paddr = allocator();
        *entry = GenericPTE::new_table(paddr);
        Some(table_of_mut(paddr))
    } else {
        next_table_mut(entry)
    }
}
