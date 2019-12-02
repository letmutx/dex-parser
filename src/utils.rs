use crate::{
    annotation::AnnotationSetItem,
    encoded_item::{EncodedItem, EncodedItemArray},
    encoded_value::EncodedValue,
    error::Error,
    jtype::{Type, TypeId},
    ushort,
};

macro_rules! try_gread_vec_with {
    ($source:ident,$offset:ident,$cap:expr,$ctx:expr) => {{
        let cap = $cap as usize;
        let ctx = $ctx;
        let mut vec = Vec::with_capacity(cap);
        // NOTE: gread_inout_with doesn't work when de-serializing encoded array
        // so using an explicit loop here.
        for _ in 0..cap {
            vec.push($source.gread_with($offset, ctx)?);
        }
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

pub(crate) fn from_item<T, F, U>(
    array: Option<EncodedItemArray<T>>,
    f: F,
) -> Option<super::Result<Vec<U>>>
where
    F: FnMut(T) -> super::Result<U>,
    T: EncodedItem,
{
    array.map(|array| array.iter().map(f).collect())
}

macro_rules! try_from_item {
    ($array:expr,$closure:expr) => {{
        use crate::utils::from_item;
        match from_item($array, $closure) {
            Some(v) => v?,
            None => Default::default(),
        }
    }};
}

pub(crate) fn get_types<S>(dex: &super::Dex<S>, type_ids: &[ushort]) -> super::Result<Vec<Type>>
where
    S: AsRef<[u8]>,
{
    type_ids
        .iter()
        .map(|type_id| dex.get_type(TypeId::from(*type_id)))
        .collect()
}

macro_rules! gen_is_flag_set {
    ($name: ident, $flag: ident) => {
        /// Returns `true` if the access flag is set
        pub fn $name(&self) -> bool {
            self.access_flags().contains(AccessFlags::$flag)
        }
    }
}

pub(crate) fn get_signature(annotations: &AnnotationSetItem) -> super::Result<Option<String>> {
    annotations
        .iter()
        .find(|item| item.jtype() == "Ldalvik/annotation/Signature;")
        .map(|item| {
            let element = item.annotation().find_element("value");
            if element.is_none() {
                return Err(Error::MalFormed(
                    "Expected element with name value, but not found".to_string(),
                ));
            }
            let element = element.unwrap();
            match *element.value() {
                EncodedValue::Array(ref v) => {
                    let signature: super::Result<String> = v
                        .iter()
                        .map(|s| {
                            if let EncodedValue::String(ref v) = s {
                                Ok(v.to_string())
                            } else {
                                Err(Error::MalFormed(format!(
                                    "Expected string element in signature, found: {:?}",
                                    s
                                )))
                            }
                        })
                        .collect();
                    Ok(Some(signature?))
                }
                ref e => Err(Error::MalFormed(format!("Expected array, found: {:?}", e))),
            }
        })
        .unwrap_or_else(|| Ok(None))
}
