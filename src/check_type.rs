use crate::{
    array::Shape,
    function::{Function, Instr},
    primitive::Primitive,
    value::Value,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ValueType {
    Num,
    Char,
    Function,
    Box(Box<Type>),
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Type {
    pub value: ValueType,
    pub shape: Option<Shape>,
}

impl Type {
    pub fn new(value: ValueType, shape: impl IntoIterator<Item = usize>) -> Self {
        Self {
            value,
            shape: Some(shape.into_iter().collect()),
        }
    }
    pub fn unknown_shape(value: ValueType) -> Self {
        Self { value, shape: None }
    }
    pub fn from_value(val: &Value) -> Self {
        Self {
            value: match val {
                Value::Num(_) | Value::Byte(_) => ValueType::Num,
                Value::Char(_) => ValueType::Char,
                Value::Func(f) => {
                    if let Some(val) = f.as_boxed() {
                        ValueType::Box(Box::new(Self::from_value(val)))
                    } else {
                        ValueType::Function
                    }
                }
            },
            shape: Some(Shape::from(val.shape())),
        }
    }
}

type TypeResult<T> = Option<T>;

pub fn check_type(f: &Function, inputs: &[Value]) -> TypeResult<Vec<Type>> {
    let mut stack = Vec::new();
    for input in inputs {
        stack.push(Type::from_value(input));
    }
    let mut env = TypeEnv {
        stack,
        array: Vec::new(),
    };
    for instr in &f.instrs {
        env.instr(instr)?;
    }
    Some(env.stack)
}

struct TypeEnv {
    stack: Vec<Type>,
    array: Vec<usize>,
}

impl TypeEnv {
    fn instr(&mut self, instr: &Instr) -> TypeResult<()> {
        match instr {
            Instr::Push(val) => self.push(Type::from_value(val)),
            Instr::BeginArray => self.array.push(self.stack.len()),
            Instr::EndArray { boxed, .. } => {
                let bottom = self.array.pop()?;
                let items = self.stack.split_off(bottom);
                let value = if items.windows(2).all(|w| w[0] == w[1]) {
                    items.get(0).map(|ty| ty.value.clone()).unwrap_or_else(|| {
                        if *boxed {
                            ValueType::Unknown
                        } else {
                            ValueType::Num
                        }
                    })
                } else {
                    ValueType::Unknown
                };
                let (value, shape) = if *boxed {
                    let value = ValueType::Box(Box::new(Type::unknown_shape(value)));
                    (value, Some(Shape::from_iter([items.len()])))
                } else {
                    let mut shape = if items.windows(2).all(|w| w[0].shape == w[1].shape) {
                        items.get(0).and_then(|ty| ty.shape.clone())
                    } else {
                        None
                    };
                    if let Some(shape) = &mut shape {
                        shape.insert(0, items.len());
                    }
                    (value, shape)
                };
                self.push(Type { value, shape });
            }
            _ => return None,
        }
        Some(())
    }
    fn prim(&mut self, prim: Primitive) -> TypeResult<()> {
        use Primitive::*;
        match prim {
            _ => return None,
        }
        Some(())
    }
    fn push(&mut self, ty: Type) {
        self.stack.push(ty);
    }
    fn pop(&mut self) -> Type {
        let val = self.stack.pop().unwrap_or_default();
        for arr in &mut self.array {
            *arr = (*arr).min(self.stack.len());
        }
        val
    }
}
