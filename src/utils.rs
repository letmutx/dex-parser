use crate::encoded_item::EncodedItem;
use crate::encoded_item::EncodedItemArray;

macro_rules! read_vec {
    ($source:ident,$offset:expr,$cap:expr,$ctx:expr) => {{
        let cap = $cap as usize;
        let mut vec = Vec::with_capacity(cap);
        unsafe {
            vec.set_len(cap);
        }
        $source.gread_inout_with($offset, &mut vec, $ctx)?;
        vec
    }};
}

macro_rules! encoded_array {
    ($source:ident,$dex:ident,$offset:ident,$size:expr) => {
        if $size > 0 {
            let encoded_array_ctx = EncodedItemArrayCtx::new($dex, $size as usize);
            Some($source.gread_with($offset, encoded_array_ctx)?)
        } else {
            None
        }
    };
}

pub(crate) fn into_item<T, F, U>(
    array: Option<EncodedItemArray<T>>,
    f: F,
) -> Option<super::Result<Vec<U>>>
where
    F: Fn(T) -> super::Result<U>,
    T: EncodedItem,
{
    array.map(|array| array.into_iter().map(f).collect())
}

macro_rules! try_into_item {
    ($array:expr,$closure:ident) => {{
        use crate::utils::into_item;
        match into_item($array, $closure) {
            Some(v) => Some(v?),
            None => None,
        }
    }};
}
