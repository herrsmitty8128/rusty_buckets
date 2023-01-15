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

    /// Returns true if the load factor is less than or equal to 0.375.
    /*#[inline]
    fn should_shrink(count: usize, capacity: usize) -> bool {
        count <= (capacity >> 2) + (capacity >> 3)
    }*/

    pub mod map {

        use std::alloc::{self, Layout};
        use std::fmt::Debug;
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

        #[derive(Clone, Debug)]
        pub struct HashTable<T>
        where
            T: Default + Copy + Clone + Debug,
        {
            count: usize,
            shift: usize,
            mask: usize,
            capacity: usize,
            ptr: *mut Bucket<T>,
        }

        impl<T> Default for HashTable<T>
        where
            T: Default + Copy + Clone + Debug,
        {
            fn default() -> Self {
                Self {
                    count: 0,
                    shift: 0,
                    mask: 0,
                    capacity: 0,
                    ptr: null_mut(),
                }
            }
        }

        impl<T> Drop for HashTable<T>
        where
            T: Default + Copy + Clone + Debug,
        {
            fn drop(&mut self) {
                let layout = Layout::array::<Bucket<T>>(self.capacity).unwrap();
                unsafe { alloc::dealloc(self.ptr as *mut u8, layout) };
            }
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

            #[inline]
            fn hash(&self, key: usize) -> usize {
                key.wrapping_mul(super::HASH_MULTIPLIER) >> self.shift
            }

            /// Returns true if the load factor greater than or equal to 0.9375.
            #[inline]
            fn should_grow(&self) -> bool {
                self.count >= (self.capacity - (self.capacity >> 4))
            }

            #[inline]
            pub fn load_factor(&self) -> f64 {
                if self.capacity == 0 {
                    0.0
                } else {
                    self.count as f64 / self.capacity as f64
                }
            }

            pub fn with_capacity(initial_capacity: usize) -> Self {
                let bits: usize = (super::USIZE_BITS - initial_capacity.leading_zeros() as usize)
                    .min(Self::MAX_BITS)
                    .max(Self::MIN_BITS);
                let capacity: usize = 1 << bits;
                let layout: Layout = Self::create_layout(capacity);
                let ptr: *mut Bucket<T> = unsafe { alloc::alloc(layout) as *mut Bucket<T> };
                if ptr.is_null() {
                    alloc::handle_alloc_error(layout);
                }
                for count in 0..capacity {
                    unsafe { *ptr.add(count) = Bucket::default() };
                }
                HashTable {
                    count: 0,
                    shift: capacity.leading_zeros() as usize + 1,
                    mask: capacity - 1,
                    capacity,
                    ptr,
                }
            }

            fn create_layout(capacity: usize) -> Layout {
                assert!(mem::size_of::<T>() != 0, "Capacity overflow");
                let layout: Layout = Layout::array::<Bucket<T>>(capacity).unwrap();
                assert!(layout.size() < isize::MAX as usize, "Allocation too large");
                layout
            }

            #[inline]
            pub fn get(&self, key: usize) -> Option<&T> {
                unsafe {
                    let h: usize = self.hash(key);
                    let mut bucket: *mut Bucket<T> = self.ptr.add(h);
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
                    let old_ptr: *mut Bucket<T> = self.ptr;
                    let old_capacity: usize = self.capacity;
                    let old_layout: Layout = Self::create_layout(old_capacity);

                    let new_cap: usize = 2 * old_capacity;
                    let new_layout: Layout = Self::create_layout(new_cap);
                    let new_ptr: *mut Bucket<T> = alloc::alloc(new_layout) as *mut Bucket<T>;

                    if new_ptr.is_null() {
                        alloc::handle_alloc_error(new_layout);
                    }

                    self.shift = new_cap.leading_zeros() as usize + 1;
                    self.mask = new_cap - 1;
                    self.capacity = new_cap;
                    self.ptr = new_ptr;

                    for count in 0..new_cap {
                        (*new_ptr.add(count)).next = null_mut();
                    }

                    for count in 0..old_capacity {
                        let b: *mut Bucket<T> = old_ptr.add(count);
                        if !(*b).next.is_null() {
                            self.emplace((*b).key, (*b).value);
                        }
                    }

                    alloc::dealloc(old_ptr as *mut u8, old_layout);
                }
            }

            #[inline]
            pub fn put(&mut self, key: usize, value: T) -> Option<T> {
                unsafe {
                    if self.should_grow() {
                        self.grow();
                    }
                    match self.emplace(key, value) {
                        Some(b) => Some(b),
                        None => {
                            self.count += 1;
                            None
                        }
                    }
                }
            }

            #[inline]
            unsafe fn emplace(&mut self, key: usize, value: T) -> Option<T> {
                let mut h: usize = self.hash(key);
                let origin: *mut Bucket<T> = self.ptr.wrapping_add(h);
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
                            h = (h + x) & self.mask;
                            next = self.ptr.wrapping_add(h);
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
                } else if h == self.hash((*origin).key) {
                    curr = origin;
                    loop {
                        if (*curr).key == key {
                            return Some(addr_of_mut!((*curr).value).replace(value));
                        }
                        if next == origin {
                            for probe in 1usize.. {
                                h = (h + probe) & self.mask;
                                next = self.ptr.wrapping_add(h);
                                if (*next).next.is_null() {
                                    *next = Bucket {
                                        next: origin,
                                        key,
                                        value,
                                    };
                                    (*curr).next = next;
                                    return None;
                                }
                            }
                        }
                        curr = next;
                        next = (*curr).next;
                    }
                } else {
                    loop {
                        curr = next;
                        next = (*curr).next;
                        if next == origin {
                            for probe in 1usize.. {
                                h = (h + probe) & self.mask;
                                next = self.ptr.wrapping_add(h);
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
                    }
                }
            }

            pub fn print(&self) {
                println!(
                    "count {}, shift {}, mask {}, cap {}, load {}",
                    self.count,
                    self.shift,
                    self.mask,
                    self.capacity,
                    self.load_factor()
                );
                /*let buckets = self.ptr.add(1) as *mut Bucket<T>;
                let mut i: usize = 0;
                while i < self.capacity {
                    println!("{:?}", buckets.add(i).read());
                    i += 1;
                }
                println!();*/
            }
        }
    }
}
