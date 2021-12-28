pub use traits::*;
pub use derive::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PartialEq, Eq, Debug)]
    pub struct NonDefault(u8);

    #[derive(QuickDefault, PartialEq, Eq, Debug)]
    pub struct QuickDefaultBaseCase {
        tuple: (u32, u32, u32),
        string: String,
        option: Option<NonDefault>,
        #[default(NonDefault(1))]
        non_default: NonDefault,
    }

    #[test]
    fn default_base_case() {
        let a = QuickDefaultBaseCase::default();
        let b = QuickDefaultBaseCase {
            tuple: Default::default(),
            string: Default::default(),
            option: Default::default(),
            non_default: NonDefault(1),
        };

        assert_eq!(a, b);
    }

    #[derive(QuickSer, PartialEq, Eq, Debug)]
    pub struct QuickSerBaseCase {
        indices: Vec<usize>,
        data: Vec<u8>,
        tuple: (u8, u8, u8),
        tuple_opt: Option<(u8, u8, u8)>,
        tuple_vec: Vec<(u8, u8, u8)>,
        tuple_vec_opt: Vec<Option<(u8, u8, u8)>>,
    }

    #[derive(Clone, Copy, RealQuickSer, PartialEq, Eq, Debug)]
    pub struct RealQuickSerBaseCase {
        tuple: (u8, u8, u8),
        uint: u8,
        uint_opt: Option<u8>,
        usize: usize,
    }

    #[test]
    fn ser_base_case() {
        let a = QuickSerBaseCase {
            indices: vec![1, 2, 3],
            data: vec![4, 5, 6],
            tuple: (7, 8, 9),
            tuple_opt: Some((10, 11, 12)),
            tuple_vec: vec![(13, 14, 15), (16, 17, 18)],
            tuple_vec_opt: vec![Some((19, 20, 21)), None],
        };

        test_ser_de(&a);
        
        let b = RealQuickSerBaseCase {
            tuple: (22, 23, 24),
            uint: 25,
            uint_opt: Some(26),
            usize: 27,
        };

        test_ser_de(&b);
    }

    fn test_ser_de<T: QuickSer + PartialEq<T> + Eq + std::fmt::Debug>(t: &T) {
        let mut buffer = Vec::new();
        t.ser(&mut buffer);
        let mut progress = 0;
        let result = T::de_ser(&mut progress, &buffer);
        assert_eq!(progress, buffer.len());
        assert_eq!(&result, t);
    }
}