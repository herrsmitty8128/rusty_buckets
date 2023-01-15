pub mod hash {

    const USIZE_BITS: usize = std::mem::size_of::<usize>() * 8;

    /// This function calculates the initial index into the hash table. It multiplies the key
    /// by a constant integral value equal to 2^64 divided by the golden ratio.
    ///
    /// The correct multiplier constant for the hash function is based on the golden ratio.
    /// The golden ratio can be calculated with Python 3 using the following statements:
    ///
    /// ```
    /// from decimal import Decimal
    /// golden_ratio = Decimal((Decimal(1.0) + Decimal.sqrt(Decimal(5.0)))/ Decimal(2.0))
    /// golden_ratio
    /// 1.618033988749894848204586834
    /// ```
    ///
    /// For 64-bit values use 2^64 / golden_ratio = 11400714819323198486
    /// For 32-bit values use 2^32 / golden_ratio = 2654435769
    const HASH_MULTIPLIER: usize = if USIZE_BITS == 64 {
        11400714819323198486
    } else if USIZE_BITS == 32 {
        2654435769
    } else {
        panic!("Only 32-bit and 64-bit platforms are supported.")
    };

    #[inline]
    fn hash(key: usize, shift: usize) -> usize {
        key.wrapping_mul(HASH_MULTIPLIER) >> shift
    }

    /// Returns true if the load factor greater than or equal to 0.9375.
    #[inline]
    fn should_grow(count: usize, capacity: usize) -> bool {
        count >= (capacity - (capacity >> 4))
    }

    /// Returns true if the load factor is less than or equal to 0.375.
    /*#[inline]
    fn should_shrink(count: usize, capacity: usize) -> bool {
        count <= (capacity >> 2) + (capacity >> 3)
    }*/

    pub mod map {

        use std::alloc::{self, Layout};
        use std::fmt::Debug;
        use std::marker::PhantomData;
        use std::mem;
        use std::ptr::{addr_of_mut, null_mut};

        #[derive(Clone, Copy, Debug)]
        struct Bucket<T>
        where
            T: Default + Debug + Copy + Clone,
        {
            next: *mut Bucket<T>,
            key: usize,
            value: T,
        }

        impl<T> Default for Bucket<T>
        where
            T: Default + Debug + Copy + Clone,
        {
            fn default() -> Self {
                Self {
                    next: null_mut(),
                    key: 0,
                    value: T::default(),
                }
            }
        }

        #[derive(Clone, Copy, Debug)]
        pub struct InnerHashTable<T>
        where
            T: Default + Copy + Clone + Debug,
        {
            count: usize,
            shift: usize,
            mask: usize,
            capacity: usize,
            body: PhantomData<Bucket<T>>,
        }

        #[derive(Clone, Debug)]
        pub struct HashTable<T>
        where
            T: Default + Copy + Clone + Debug,
        {
            ptr: *mut InnerHashTable<T>,
        }

        impl<T> HashTable<T>
        where
            T: Default + Copy + Clone + Debug,
        {
            pub const BUCKET_SIZE: usize = std::mem::size_of::<Bucket<T>>();
            pub const MIN_BITS: usize = 1;
            pub const MAX_BITS: usize =
                super::USIZE_BITS - (usize::MAX / Self::BUCKET_SIZE).leading_zeros() as usize;
            pub const MIN_CAPACITY: usize = 1 << Self::MIN_BITS;
            pub const MAX_CAPACITY: usize = 1 << Self::MAX_BITS;

            pub fn with_capacity(initial_capacity: usize) -> Self {
                let bits: usize = (super::USIZE_BITS - initial_capacity.leading_zeros() as usize)
                    .min(Self::MAX_BITS)
                    .max(Self::MIN_BITS);
                let capacity: usize = 1 << bits;
                let layout: Layout = Self::create_layout(capacity);
                let ptr: *mut InnerHashTable<T> =
                    unsafe { alloc::alloc(layout) as *mut InnerHashTable<T> };
                if ptr.is_null() {
                    alloc::handle_alloc_error(layout);
                }
                unsafe {
                    ptr.write(InnerHashTable {
                        count: 0,
                        shift: capacity.leading_zeros() as usize + 1,
                        mask: capacity - 1,
                        capacity: capacity as usize,
                        body: PhantomData,
                    });

                    let mut i: usize = 0;
                    let buckets: *mut Bucket<T> = ptr.add(1) as *mut Bucket<T>;
                    while i < capacity {
                        *buckets.add(i) = Bucket::default();
                        i += 1;
                    }
                }
                HashTable { ptr }
            }

            fn create_layout(capacity: usize) -> Layout {
                assert!(mem::size_of::<T>() != 0, "Capacity overflow");
                let head_layout: Layout = Layout::new::<InnerHashTable<T>>();
                let body_layout: Layout = Layout::array::<Bucket<T>>(capacity).unwrap();
                let size: usize = head_layout.size() + body_layout.size();
                assert!(size < isize::MAX as usize, "Allocation too large");
                let align: usize = head_layout.align().max(body_layout.align());
                let new_layout: Layout = Layout::from_size_align(size, align).unwrap();
                new_layout
            }

            #[inline]
            pub fn get(&self, key: usize) -> Option<&T> {
                unsafe {
                    let h: usize = super::hash(key, (*self.ptr).shift);
                    let mut bucket: *mut Bucket<T> = (self.ptr.add(1) as *mut Bucket<T>).add(h);
                    let origin: *mut Bucket<T> = bucket;
                    loop {
                        if (*bucket).key == key {
                            return Some(&(*bucket).value);
                        }
                        bucket = (*bucket).next;
                        if bucket == origin {
                            return None;
                        }
                    }
                }
            }

            fn grow(&mut self) {
                unsafe {
                    let old_ptr: *mut InnerHashTable<T> = self.ptr;
                    let old_capacity: usize = (*old_ptr).capacity;
                    let old_layout: Layout = Self::create_layout(old_capacity);

                    let new_cap: usize = 2 * old_capacity;
                    let new_layout: Layout = Self::create_layout(new_cap);
                    let mut new_ptr: *mut InnerHashTable<T> =
                        alloc::alloc(new_layout) as *mut InnerHashTable<T>;

                    if new_ptr.is_null() {
                        alloc::handle_alloc_error(new_layout);
                    }

                    (*new_ptr).count = (*old_ptr).count;
                    (*new_ptr).shift = new_cap.leading_zeros() as usize + 1;
                    (*new_ptr).mask = new_cap - 1;
                    (*new_ptr).capacity = new_cap;
                    self.ptr = new_ptr;

                    let mut i: usize = 0;
                    let mut buckets: *mut Bucket<T> = new_ptr.add(1) as *mut Bucket<T>;
                    while i < new_cap {
                        (*buckets.add(i)).next = null_mut();
                        i += 1;
                    }

                    i = 0;
                    buckets = old_ptr.add(1) as *mut Bucket<T>;
                    while i < old_capacity {
                        let b: *mut Bucket<T> = buckets.add(i);
                        if !(*b).next.is_null() {
                            self.emplace((*b).key, (*b).value);
                        }
                        i += 1;
                    }

                    alloc::dealloc(old_ptr as *mut u8, old_layout);
                }
            }

            #[inline]
            pub fn put(&mut self, key: usize, value: T) -> Option<T> {
                unsafe {
                    if super::should_grow((*self.ptr).count, (*self.ptr).capacity) {
                        self.grow();
                    }
                    match self.emplace(key, value) {
                        Some(b) => Some(b),
                        None => {
                            (*self.ptr).count += 1;
                            None
                        }
                    }
                }
            }

            #[inline]
            unsafe fn emplace(&mut self, key: usize, value: T) -> Option<T> {
                let mut h: usize = super::hash(key, (*self.ptr).shift);
                let buckets: *mut Bucket<T> = self.ptr.add(1) as *mut Bucket<T>;
                let origin: *mut Bucket<T> = buckets.add(h);
                let mut next: *mut Bucket<T> = (*origin).next;

                let mut curr: *mut Bucket<T>;

                if next.is_null() {
                    *origin = Bucket {
                        next: origin,
                        key,
                        value,
                    };
                    None
                } else if next == origin {
                    if (*origin).key == key {
                        Some(addr_of_mut!((*origin).value).replace(value))
                    } else {
                        for x in 1usize.. {
                            h = (h + x) & (*self.ptr).mask;
                            next = buckets.add(h);
                            if (*next).next.is_null() {
                                *next = Bucket {
                                    next: origin,
                                    key,
                                    value,
                                };
                                (*origin).next = next;
                                break;
                            }
                        }
                        None
                    }
                } else if h == super::hash((*origin).key, (*self.ptr).shift) {
                    curr = origin;
                    loop {
                        if (*curr).key == key {
                            return Some(addr_of_mut!((*curr).value).replace(value));
                        }
                        if next == origin {
                            for x in 1usize.. {
                                h = (h + x) & (*self.ptr).mask;
                                next = buckets.add(h);
                                if (*next).next.is_null() {
                                    (*curr).next = next;
                                    *next = Bucket {
                                        next: origin,
                                        key,
                                        value,
                                    };
                                    return None;
                                }
                            }
                        }
                        curr = next;
                        next = (*curr).next;
                    }
                } else {
                    curr = next;
                    loop {
                        next = (*curr).next;
                        if next == origin {
                            for x in 1usize.. {
                                h = (h + x) & (*self.ptr).mask;
                                next = buckets.add(h);
                                if (*next).next.is_null() {
                                    *next = *origin;
                                    (*curr).next = next;
                                    *origin = Bucket {
                                        next: origin,
                                        key,
                                        value,
                                    };
                                    return None;
                                }
                            }
                        }
                        curr = next;
                    }
                }

            }

            pub fn print(&self) {
                unsafe {
                    println!(
                        "count {}, shift {}, mask {}, cap {}", // load {}",
                        (*self.ptr).count,
                        (*self.ptr).shift,
                        (*self.ptr).mask,
                        (*self.ptr).capacity,
                        //self.load_factor()
                    );
                    let buckets = self.ptr.add(1) as *mut Bucket<T>;
                    let mut i: usize = 0;
                    while i < (*self.ptr).capacity {
                        println!("{:?}", buckets.add(i).read());
                        i += 1;
                    }
                    println!();
                }
            }
        }
    }
}
