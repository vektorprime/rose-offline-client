use std::{any::Any, cmp::Ordering, collections::HashMap, sync::Arc};

use num_traits::ToPrimitive;

use crate::scripting::lua4::Lua4Function;

#[derive(Clone, Debug)]
pub enum Lua4Value {
    Nil,
    UserData(Arc<dyn Any + Send + Sync>),
    Number(f64),
    String(Arc<str>),
    Table {
        fields: HashMap<Arc<str>, Lua4Value>,
        array: Vec<Lua4Value>,
    },
    Closure(Arc<Lua4Function>, Arc<Vec<Lua4Value>>),
    RustClosure(String),
}

impl Lua4Value {
    pub fn to_user_type<T: Any>(&self) -> Result<&T, LuaValueConversionError> {
        if let Lua4Value::UserData(user_data) = self {
            user_data
                .downcast_ref::<T>()
                .ok_or(LuaValueConversionError::InvalidType)
        } else {
            Err(LuaValueConversionError::InvalidType)
        }
    }

    pub fn to_i32(&self) -> Result<i32, LuaValueConversionError> {
        self.try_into()
    }

    pub fn to_usize(&self) -> Result<usize, LuaValueConversionError> {
        self.try_into()
    }

    pub fn to_string(&self) -> Result<String, LuaValueConversionError> {
        self.try_into()
    }
}

impl PartialEq for Lua4Value {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Lua4Value::Nil => matches!(other, Lua4Value::Nil),
            Lua4Value::Number(value) => {
                if let Lua4Value::Number(other) = other {
                    value == other
                } else {
                    false
                }
            }
            Lua4Value::String(value) => {
                if let Lua4Value::String(other) = other {
                    value == other
                } else {
                    false
                }
            }
            Lua4Value::Table { fields, array } => {
                if let Lua4Value::Table {
                    fields: other_fields,
                    array: other_array,
                } = other
                {
                    fields == other_fields && array == other_array
                } else {
                    false
                }
            }
            Lua4Value::UserData(_) => false,
            Lua4Value::Closure(_, _) => false,
            Lua4Value::RustClosure(_) => false,
        }
    }
}

impl Eq for Lua4Value {}

impl PartialOrd for Lua4Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            Lua4Value::Number(value) => {
                if let Lua4Value::Number(other) = other {
                    value.partial_cmp(other)
                } else {
                    None
                }
            }
            Lua4Value::String(value) => {
                if let Lua4Value::String(other) = other {
                    value.partial_cmp(other)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum LuaValueConversionError {
    InvalidType,
}

impl From<bool> for Lua4Value {
    fn from(value: bool) -> Self {
        Lua4Value::Number(if value { 1.0 } else { 0.0 })
    }
}

impl From<i32> for Lua4Value {
    fn from(value: i32) -> Self {
        Lua4Value::Number(value as f64)
    }
}

impl From<usize> for Lua4Value {
    fn from(value: usize) -> Self {
        Lua4Value::Number(value as f64)
    }
}

impl From<f32> for Lua4Value {
    fn from(value: f32) -> Self {
        Lua4Value::Number(value as f64)
    }
}

impl From<f64> for Lua4Value {
    fn from(value: f64) -> Self {
        Lua4Value::Number(value)
    }
}

impl From<String> for Lua4Value {
    fn from(value: String) -> Self {
        Lua4Value::String(value.into())
    }
}

macro_rules! impl_try_from {
    ($($ty:ty: number = |$num:ident| $num_expr:expr, string = |$str:ident| $str_expr:expr),+ $(,)?) => {
        $(
            impl TryFrom<&Lua4Value> for $ty {
                type Error = LuaValueConversionError;

                fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
                    match value {
                        Lua4Value::Number($num) => $num_expr,
                        Lua4Value::String($str) => $str_expr,
                        _ => Err(LuaValueConversionError::InvalidType),
                    }
                }
            }
        )+
    };
}

impl_try_from!(
    f32: number = |number| Ok(*number as f32), string = |string| string.parse::<f64>().map(|value| value as f32).map_err(|_| LuaValueConversionError::InvalidType),
    f64: number = |number| Ok(*number), string = |string| string.parse::<f64>().map_err(|_| LuaValueConversionError::InvalidType),
    i32: number = |number| number.to_i32().ok_or(LuaValueConversionError::InvalidType), string = |string| string.parse::<f64>().map(|value| value as i32).map_err(|_| LuaValueConversionError::InvalidType),
    i64: number = |number| number.to_i64().ok_or(LuaValueConversionError::InvalidType), string = |string| string.parse::<f64>().map(|value| value as i64).map_err(|_| LuaValueConversionError::InvalidType),
    usize: number = |number| number.to_usize().ok_or(LuaValueConversionError::InvalidType), string = |string| string.parse::<usize>().map_err(|_| LuaValueConversionError::InvalidType),
);

impl TryFrom<&Lua4Value> for String {
    type Error = LuaValueConversionError;

    fn try_from(value: &Lua4Value) -> Result<Self, Self::Error> {
        match value {
            Lua4Value::Number(number) => Ok(format!("{}", *number)),
            Lua4Value::String(string) => Ok(string.to_string()),
            _ => Err(LuaValueConversionError::InvalidType),
        }
    }
}
