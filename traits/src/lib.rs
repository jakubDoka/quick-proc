use std::{marker::PhantomData, time::SystemTime, collections::HashMap, hash::Hash};


pub trait QuickSer where Self: Sized {
    const QUICK: bool = false;

    fn ser(&self, buffer: &mut Vec<u8>);

    fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self;
}

pub trait RealQuickSer: Copy {}

impl<T> RealQuickSer for PhantomData<T> {}

impl QuickSer for String {
    fn ser(&self, buffer: &mut Vec<u8>) {
        self.len().ser(buffer);
        buffer.extend_from_slice(self.as_bytes());
    }

    fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
        let len = usize::de_ser(progress, buffer);
        let mut result = String::with_capacity(len);
        result.push_str(unsafe { 
            std::str::from_utf8_unchecked(&buffer[*progress..*progress + len]) 
        });
        *progress += len;
        result
    }
}

impl<T: QuickSer> QuickSer for Option<T> {
    fn ser(&self, buffer: &mut Vec<u8>) {
        match self {
            Some(t) => {
                buffer.push(1);
                t.ser(buffer);
            },
            None => {
                buffer.push(0);
            },
        }
    }

    fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
        match buffer[*progress] {
            0 => {
                *progress += 1;
                None
            },
            1 => {
                *progress += 1;
                Some(T::de_ser(progress, buffer))
            },
            _ => panic!("invalid enum tag"),
        }
    }
}

impl<K: QuickSer + Eq + Hash, V: QuickSer> QuickSer for HashMap<K, V> {
    fn ser(&self, buffer: &mut Vec<u8>) {
        self.len().ser(buffer);
        for (k, v) in self.iter() {
            k.ser(buffer);
            v.ser(buffer);
        }
    }

    fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
        let len = usize::de_ser(progress, buffer);
        let mut result = HashMap::with_capacity(len);
        for _ in 0..len {
            let k = K::de_ser(progress, buffer);
            let v = V::de_ser(progress, buffer);
            result.insert(k, v);
        }
        result
    }
}

impl<T: QuickSer> QuickSer for Vec<T> {
    fn ser(&self, buffer: &mut Vec<u8>) {
        if T::QUICK {
            self.len().ser(buffer);
            let len = self.len() * std::mem::size_of::<T>();
            let new_len = len + buffer.len();
            buffer.reserve(new_len);
            unsafe {
                buffer.set_len(new_len);
                std::ptr::copy_nonoverlapping(
                    self.as_ptr() as *const u8, 
                    buffer.as_mut_ptr().offset((buffer.len() - len) as isize), 
                    len
                );
            }
        } else {
            self.len().ser(buffer);
            for item in self {
                item.ser(buffer);
            }
        }
    }

    fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
        if T::QUICK {
            let len = usize::de_ser(progress, buffer);
            let mut result = Vec::with_capacity(len);
            let true_len = len * std::mem::size_of::<T>();
            unsafe {
                result.set_len(len);
                std::ptr::copy_nonoverlapping(
                    buffer.as_ptr().offset(*progress as isize),
                    result.as_mut_ptr() as *mut u8,
                    true_len
                );
            }
            *progress += true_len;
            result
        } else {
            let mut result = Vec::with_capacity(usize::de_ser(progress, buffer));
            for _ in 0..result.capacity() {
                result.push(T::de_ser(progress, buffer));
            }
            result
        }
    }
}

#[macro_export]
macro_rules! gen_quick_copy {
    () => {
        fn ser(&self, buffer: &mut Vec<u8>) {
            let size = std::mem::size_of::<Self>();
            let new_len = buffer.len() + size;
            buffer.reserve(new_len);
            unsafe {
                buffer.set_len(new_len);
                std::ptr::write(
                    buffer.as_mut_ptr().offset((buffer.len() - size) as isize) as *mut Self,
                    self.to_owned()
                );
            }
        }
    
        fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
            let size = std::mem::size_of::<Self>();
            let result = unsafe {
                std::ptr::read(
                    buffer.as_ptr().offset(*progress as isize) as *const Self
                )
            };
            *progress += size;
            result
        }
    };
}

impl<T: RealQuickSer> QuickSer for T {
    const QUICK: bool = true;

    gen_quick_copy!();
}

macro_rules! impl_traits_for_types {
    ($($integer:ty),*) => {
        $(
            impl RealQuickSer for $integer {}
        )*
    };
}

impl_traits_for_types!(
    u8, u16, u32, u64, u128, usize, 
    i8, i16, i32, i64, i128, isize,
    f32, f64, bool, char, SystemTime
);

macro_rules! impl_traits_for_tuples {
    ($(($($type:ident),*)),*) => {
        $(
            impl<$($type: RealQuickSer),*> RealQuickSer for ($($type),*) {}
        )*
    }
}

// seems enough to me
impl_traits_for_tuples!(
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H)
);