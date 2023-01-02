pub mod hash_table {
    
    const HEAD_BIT_MASK: usize = 9223372036854775808;
    const EMPTY_BIT_MASK: usize = 4611686018427387904;
    const PROBE_BITS_MASK: usize = 4611686018427387903;
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
    #[inline]
    fn hash(key: usize, shift: usize) -> usize {
        key.wrapping_mul(2654435769) >> shift
    }

    /// Is the load factor greater than or equal to 0.9375?
    #[inline]
    fn should_grow(count: usize, capacity: usize) -> bool {
        count >= (capacity - (capacity >> 4))
    }

    /// Is the load factor is less than 0.375?
    #[inline]
    fn should_shrink(count: usize, capacity: usize) -> bool {
        count <= (capacity >> 2) + (capacity >> 3)
    }

    pub mod hash_map {

        use std::fmt::Debug;
        use std::ptr::null_mut;

        #[derive(Debug, Copy, Clone)]
            struct Bucket<T>
            where
                T: Debug + Clone + Copy + Default,
            {
                meta: usize,
                key: usize,
                value: T,
            }

            impl<T> Default for Bucket<T>
            where
                T: Debug + Clone + Copy + Default,
            {
                fn default() -> Self {
                    Self {
                        meta: super::EMPTY_BIT_MASK,
                        key: 0,
                        value: T::default(),
                    }
                }
            }

            #[derive(Debug, Clone)]
            pub struct HashTable<T>
            where
                T: Debug + Clone + Copy + Default,
            {
                count: usize,
                shift: usize,
                mask: usize,
                buckets: Vec<Bucket<T>>,
            }

            impl<T> HashTable<T>
        where
            T: Debug + Clone + Copy + Default,
        {
            pub const BUCKET_SIZE: usize = std::mem::size_of::<Bucket<T>>();
            pub const MIN_BITS: usize = 1;
            pub const MAX_BITS: usize =
                super::USIZE_BITS - (usize::MAX / Self::BUCKET_SIZE).leading_zeros() as usize;
            pub const MIN_CAPACITY: usize = 1 << Self::MIN_BITS;
            pub const MAX_CAPACITY: usize = 1 << Self::MAX_BITS;

            pub fn load_factor(&self) -> f64 {
                if self.capacity() == 0 {
                    0.0
                } else {
                    self.count as f64 / self.capacity() as f64
                }
            }
        
            pub fn count(&self) -> usize {
                self.count
            }

            pub fn capacity(&self) -> usize {
                self.buckets.len()
            }

            pub fn with_capacity(initial_capacity: usize) -> HashTable<T> {
                let bits: usize = (super::USIZE_BITS - initial_capacity.leading_zeros() as usize)
                    .min(Self::MAX_BITS)
                    .max(Self::MIN_BITS);
                let capacity: usize = 1 << bits;
                HashTable {
                    count: 0,
                    shift: super::USIZE_BITS - bits,
                    mask: capacity - 1,
                    buckets: vec![Bucket::<T>::default(); capacity],
                }
            }

            pub fn get(&mut self, key: usize) -> Option<&T> {
                unsafe {
                    let h: usize = super::hash(key, self.shift);
                    let buckets: *mut Bucket<T> = self.buckets.as_mut_ptr();
                    let mut bucket: *mut Bucket<T> = buckets.add(h);
                    let mut n: usize = bucket.read().meta;
                    if n & super::HEAD_BIT_MASK != 0 {
                        n ^= super::HEAD_BIT_MASK;
                        loop {
                            if bucket.read().key == key {
                                return Some(&(*bucket).value);
                            }
                            if n == h {
                                break;
                            }
                            bucket = buckets.add(n);
                            n = bucket.read().meta;
                        }
                    }
                    None
                }
            }

            pub fn emplace(&mut self, key: usize, value: T) -> Option<T> {
                unsafe {
                    let h: usize = super::hash(key, self.shift);
                    let buckets: *mut Bucket<T> = self.buckets.as_mut_ptr();
                    let mut bucket: *mut Bucket<T> = buckets.add(h);
                    let mut n: usize = bucket.read().meta;
                    let mut i: usize;
                    if n & super::EMPTY_BIT_MASK != 0 {
                        bucket.write(Bucket {
                            meta: super::HEAD_BIT_MASK | h,
                            key,
                            value,
                        });
                    } else if n & super::HEAD_BIT_MASK != 0 {
                        n ^= super::HEAD_BIT_MASK;
                        i = h;
                        loop {
                            if bucket.read().key == key {
                                let v: T = (*bucket).value;
                                (*bucket).value = value;
                                return Some(v);
                            }
                            if n == h {
                                break;
                            }
                            i = n;
                            bucket = buckets.add(i);
                            n = bucket.read().meta;
                        }
                        n = 1;
                        while n < self.capacity() {
                            i += n;
                            i &= self.mask;
                            let empty: *mut Bucket<T> = buckets.add(i);
                            if empty.read().meta & super::EMPTY_BIT_MASK != 0 {
                                (*bucket).meta = (bucket.read().meta & super::HEAD_BIT_MASK) | i;
                                empty.write(Bucket {
                                    meta: h,
                                    key,
                                    value,
                                });
                                break;
                            }
                            n += 1;
                        }
                    } else {
                        loop {
                            i = n;
                            n = buckets.add(i).read().meta & super::PROBE_BITS_MASK;
                            if n == h {
                                break;
                            }
                        }
                        let last: *mut usize = buckets.add(i) as *mut usize;
                        n = 1;
                        while n < self.capacity() {
                            i += n;
                            i &= self.mask;
                            let empty: *mut Bucket<T> = buckets.add(i);
                            
                            if empty.read().meta & super::EMPTY_BIT_MASK != 0 {

                                // point the current bucket to the empty bucket to remove h
                                last.write((last.read() & super::HEAD_BIT_MASK) | i);
                                
                                // move h to the empty bucket and the key and value to h
                                empty.write(bucket.replace(Bucket {
                                    meta: super::HEAD_BIT_MASK | h,
                                    key,
                                    value,
                                }));
                                break;
                            }
                            n += 1;
                        }
                    }
                    None
                }
            }

            pub fn insert(&mut self, key: usize, value: T) -> Option<T> {
                if super::should_grow(self.count, self.capacity()) {
                    self.grow();
                }
                match self.emplace(key, value) {
                    Some(x) => Some(x),
                    None => {
                        self.count += 1;
                        None
                    }
                }
            }

            pub fn delete(&mut self, key: usize) {
                unsafe {
                    let buckets: *mut Bucket<T> = self.buckets.as_mut_ptr();
                    let h: usize = super::hash(key, self.shift);
                    let mut erase: *mut Bucket<T> = null_mut();
                    let mut last: *mut Bucket<T> = buckets.add(h);
                    let mut prev_meta: *mut usize = null_mut();
                    let mut meta: usize =  last.cast::<usize>().read(); //(*last).meta;

                    if meta & super::HEAD_BIT_MASK != 0 {
                        meta ^= super::HEAD_BIT_MASK;
                        loop {
                            if (*last).key == key {
                                erase = last;
                            }
                            if meta == h {
                                break;
                            };
                            prev_meta = last.cast::<usize>(); //last as *mut usize;
                            last = buckets.add(meta);
                            meta = last.cast::<usize>().read(); //(*last).meta;
                        }
                        if !erase.is_null() {
                            if !prev_meta.is_null() {
                                *prev_meta = (*prev_meta & super::HEAD_BIT_MASK) | h;
                            }
                            (*erase).key = (*last).key;
                            (*erase).value = (*last).value;
                            (*last).meta = super::EMPTY_BIT_MASK;
                            self.count -= 1;
                            if super::should_shrink(self.count, self.capacity()) {
                                self.shrink();
                            }
                        }
                    }
                };
            }

            fn rehash_non_zero(&mut self, h: usize) {
                unsafe {
                    let buckets: *mut Bucket<T> = self.buckets.as_mut_ptr();
                    let bucket: *mut Bucket<T> = buckets.add(h);
                    if bucket.read().meta & super::HEAD_BIT_MASK != 0 {
                    //if (*bucket).meta & HEAD_BIT_MASK != 0 {
                        loop {
                            self.emplace(
                                // will always hash to an index != h
                                bucket.read().key, //(*bucket).key,
                                bucket.read().value, //(*bucket).value,
                            );
                            let n: usize = (*bucket).meta ^ super::HEAD_BIT_MASK;
                            if n == h {
                                break;
                            }
                            let next: *mut Bucket<T> = buckets.add(n);
                            //bucket.copy_from(next, 1);
                            bucket.copy_from_nonoverlapping(next, 1); // copy n into h
                            //(*next).meta = EMPTY_BIT_MASK;
                            next.cast::<usize>().write(super::EMPTY_BIT_MASK); // mark n as empty
                            //(*bucket).meta |= HEAD_BIT_MASK;
                            bucket.cast::<usize>().write(bucket.read().meta | super::HEAD_BIT_MASK); // turn on the header bit for h
                        }
                        //(*bucket).meta = EMPTY_BIT_MASK;
                        bucket.cast::<usize>().write(super::EMPTY_BIT_MASK);
                    }
                };
            }

            fn rehash_zero(&mut self) {
                unsafe {
                    let buckets: *mut Bucket<T> = self.buckets.as_mut_ptr();
                    if (*buckets).meta & super::HEAD_BIT_MASK != 0 {
                        let mut p: usize = 0;
                        let mut i: usize = (*buckets).meta & super::PROBE_BITS_MASK;
                        while i != 0 {
                            let n: usize = (*buckets.add(i)).meta & super::PROBE_BITS_MASK;
                            let h2: usize = super::hash(buckets.add(i).read().key, self.shift);
                            if h2 != 0 {
                                (*buckets.add(p)).meta =
                                    ((*buckets.add(p)).meta & super::HEAD_BIT_MASK) | n;
                                (*buckets.add(i)).meta = super::EMPTY_BIT_MASK;
                                self.emplace((*buckets.add(i)).key, (*buckets.add(i)).value);
                                i = if h2 == n {
                                    (*buckets.add(p)).meta & super::PROBE_BITS_MASK
                                } else {
                                    n
                                };
                            } else {
                                p = i;
                                i = n;
                            }
                        }
                        let h2: usize = super::hash(buckets.add(i).read().key, self.shift);
                        if h2 != 0 {
                            let n: usize = (*buckets.add(i)).meta & super::PROBE_BITS_MASK;
                            if n != 0 {
                                // there are other buckets in the list that were not removed
                                // swap i and n;
                                buckets.add(i).swap(buckets.add(n));
                                // fix-up the meta data for i and n
                                (*buckets.add(i)).meta |= super::HEAD_BIT_MASK;
                                (*buckets.add(n)).meta = super::EMPTY_BIT_MASK;
                                self.emplace((*buckets.add(n)).key, (*buckets.add(n)).value);
                            } else {
                                // i is the last remaining bucket in the list
                                (*buckets.add(i)).meta = super::EMPTY_BIT_MASK;
                                self.emplace((*buckets.add(i)).key, (*buckets.add(i)).value);
                            }
                        }
                    }
                };
            }

            fn grow(&mut self) {
                let old_capacity: usize = self.capacity();
                let new_capacity: usize = old_capacity * 2;
                if new_capacity.is_power_of_two() && new_capacity <= Self::MAX_CAPACITY {
                    self.buckets.resize(new_capacity, Bucket::<T>::default());
                    self.mask = self.capacity() - 1;
                    self.shift -= 1;
                    let mut max: usize = old_capacity;
                    let mut min: usize = max >> 1;
                    while min > 0 {
                        while min < max {
                            self.rehash_non_zero(min);
                            min += 1;
                        }
                        max >>= 1;
                        min = max >> 1;
                    }
                    self.rehash_zero();
                }
            }

            fn shrink(&mut self) {
                let old_capacity: usize = self.capacity();
                let new_capacity: usize = old_capacity / 2;
                if new_capacity.is_power_of_two() && new_capacity >= Self::MIN_CAPACITY {
                    self.mask = new_capacity - 1;
                    self.shift += 1;
                    let mut i: usize = 1;
                    while i < old_capacity {
                        self.rehash_non_zero(i);
                        i += 1;
                    }
                    self.buckets.truncate(new_capacity);
                }
            }

        }

    }

}