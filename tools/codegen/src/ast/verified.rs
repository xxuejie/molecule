use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use super::raw::{Ast as RawAst, TopDecl as RawTopDecl};

pub(crate) const ATOM_NAME: &str = "byte";
pub(crate) const ATOM_SIZE: usize = 1;

#[derive(Debug)]
pub(crate) struct Ast {
    pub(crate) decls: Vec<Rc<TopDecl>>,
}

#[derive(Debug)]
pub(crate) struct TopDecl {
    pub(crate) name: String,
    pub(crate) typ: TopDeclType,
}

#[derive(Debug)]
pub(crate) enum TopDeclType {
    Atom,
    Option_(Option_),
    Union(Union),
    Array(Array),
    Struct(Struct),
    FixedVector(FixedVector),
    DynamicVector(DynamicVector),
    Table(Table),
}

#[derive(Debug)]
pub(crate) struct Option_ {
    pub(crate) typ: Rc<TopDecl>,
}

#[derive(Debug)]
pub(crate) struct Union {
    pub(crate) inner: Vec<ItemDecl>,
}

#[derive(Debug)]
pub(crate) struct Array {
    pub(crate) item_size: usize,
    pub(crate) item_count: usize,
    pub(crate) typ: Rc<TopDecl>,
}

#[derive(Debug)]
pub(crate) struct Struct {
    pub(crate) field_size: Vec<usize>,
    pub(crate) inner: Vec<FieldDecl>,
}

#[derive(Debug)]
pub(crate) struct FixedVector {
    pub(crate) item_size: usize,
    pub(crate) typ: Rc<TopDecl>,
}

#[derive(Debug)]
pub(crate) struct DynamicVector {
    pub(crate) typ: Rc<TopDecl>,
}

#[derive(Debug)]
pub(crate) struct Table {
    pub(crate) content: Vec<u8>,
    pub(crate) inner: Vec<FieldDecl>,
}

#[derive(Debug)]
pub(crate) struct ItemDecl {
    pub(crate) typ: Rc<TopDecl>,
}

#[derive(Debug)]
pub(crate) struct FieldDecl {
    pub(crate) name: String,
    pub(crate) typ: Rc<TopDecl>,
}

macro_rules! impl_top_decl_type_for {
    ($type:ident) => {
        impl From<$type> for TopDeclType {
            fn from(typ: $type) -> Self {
                TopDeclType::$type(typ)
            }
        }
    };
}

impl_top_decl_type_for!(Option_);
impl_top_decl_type_for!(Union);
impl_top_decl_type_for!(Array);
impl_top_decl_type_for!(Struct);
impl_top_decl_type_for!(FixedVector);
impl_top_decl_type_for!(DynamicVector);
impl_top_decl_type_for!(Table);

impl Option_ {
    pub fn default_content(&self) -> Vec<u8> {
        Vec::new()
    }
}

impl Union {
    pub fn default_content(&self) -> Vec<u8> {
        let v: molecule::ItemId = 0;
        (&v.to_le_bytes()[..]).to_owned()
    }
}

impl Array {
    pub fn default_content(&self) -> Vec<u8> {
        vec![0; self.item_size * self.item_count]
    }
}

impl Struct {
    pub fn default_content(&self) -> Vec<u8> {
        vec![0; self.field_size.iter().sum()]
    }
}

impl FixedVector {
    pub fn default_content(&self) -> Vec<u8> {
        (&0u32.to_le_bytes()[..]).to_owned()
    }
}

impl DynamicVector {
    pub fn default_content(&self) -> Vec<u8> {
        (&4u32.to_le_bytes()[..]).to_owned()
    }
}

impl Table {
    pub fn default_content(&self) -> Vec<u8> {
        self.content.clone()
    }
}

impl TopDecl {
    fn new(name: &str, typ: impl Into<TopDeclType>) -> Self {
        Self {
            name: name.to_owned(),
            typ: typ.into(),
        }
    }

    fn atom() -> Self {
        TopDecl {
            name: ATOM_NAME.to_owned(),
            typ: TopDeclType::Atom,
        }
    }

    pub(crate) fn is_atom(&self) -> bool {
        match self.typ {
            TopDeclType::Atom => true,
            _ => false,
        }
    }

    fn total_size(&self) -> Option<usize> {
        match self.typ {
            TopDeclType::Atom => Some(ATOM_SIZE),
            TopDeclType::Option_(_) => None,
            TopDeclType::Union(_) => None,
            TopDeclType::Array(ref typ) => Some(typ.item_size * typ.item_count),
            TopDeclType::Struct(ref typ) => Some(typ.field_size.iter().sum()),
            TopDeclType::FixedVector(_) => None,
            TopDeclType::DynamicVector(_) => None,
            TopDeclType::Table(_) => None,
        }
    }

    fn default_content(&self) -> Vec<u8> {
        match self.typ {
            TopDeclType::Atom => vec![0],
            TopDeclType::Option_(ref typ) => typ.default_content(),
            TopDeclType::Union(ref typ) => typ.default_content(),
            TopDeclType::Array(ref typ) => typ.default_content(),
            TopDeclType::Struct(ref typ) => typ.default_content(),
            TopDeclType::FixedVector(ref typ) => typ.default_content(),
            TopDeclType::DynamicVector(ref typ) => typ.default_content(),
            TopDeclType::Table(ref typ) => typ.default_content(),
        }
    }

    fn complete(raw: &RawTopDecl, deps: &HashMap<&str, Rc<Self>>) -> Option<Self> {
        match raw {
            RawTopDecl::Option_(raw_decl) => {
                if let Some(dep) = deps.get(raw_decl.typ.as_str()) {
                    let typ = Rc::clone(dep);
                    let typ: TopDeclType = Option_ { typ }.into();
                    Some(TopDecl::new(raw.name(), typ))
                } else {
                    None
                }
            }
            RawTopDecl::Union(raw_decl) => {
                let mut inner = Vec::with_capacity(raw_decl.inner.len());
                for raw_item in &raw_decl.inner[..] {
                    if let Some(dep) = deps.get(raw_item.typ.as_str()) {
                        let item = ItemDecl {
                            typ: Rc::clone(dep),
                        };
                        inner.push(item);
                    } else {
                        break;
                    }
                }
                if inner.len() == raw_decl.inner.len() {
                    let typ: TopDeclType = Union { inner }.into();
                    Some(TopDecl::new(raw.name(), typ))
                } else {
                    None
                }
            }
            RawTopDecl::Array(raw_decl) => {
                if let Some(dep) = deps.get(raw_decl.typ.as_str()) {
                    let typ = Rc::clone(dep);
                    let item_count = raw_decl.length;
                    let typ: TopDeclType = if let Some(item_size) = dep.total_size() {
                        if item_size == 0 {
                            panic!("the array ({}) has no size", raw.name());
                        }
                        Array {
                            item_size,
                            item_count,
                            typ,
                        }
                        .into()
                    } else {
                        panic!(
                            "the inner type ({}) of array ({}) doesn't have fixed size",
                            raw_decl.typ,
                            raw.name()
                        );
                    };
                    Some(TopDecl::new(raw.name(), typ))
                } else {
                    None
                }
            }
            RawTopDecl::Struct(raw_decl) => {
                let mut inner = Vec::with_capacity(raw_decl.inner.len());
                let mut field_size = Vec::with_capacity(raw_decl.inner.len());
                for raw_field in &raw_decl.inner[..] {
                    let field_name = raw_field.name.to_owned();
                    if let Some(dep) = deps.get(raw_field.typ.as_str()) {
                        if let Some(item_size) = dep.total_size() {
                            field_size.push(item_size);
                        } else {
                            panic!(
                                "the inner type ({}) in struct ({}) doesn't have fixed size",
                                field_name,
                                raw.name()
                            );
                        }
                        let field = FieldDecl {
                            name: field_name,
                            typ: Rc::clone(dep),
                        };
                        inner.push(field);
                    } else {
                        break;
                    }
                }
                if inner.len() == raw_decl.inner.len() {
                    if field_size.iter().sum::<usize>() == 0 {
                        panic!("the struct ({}) has no size", raw.name());
                    }
                    let typ: TopDeclType = Struct { field_size, inner }.into();
                    Some(TopDecl::new(raw.name(), typ))
                } else {
                    None
                }
            }
            RawTopDecl::Vector(raw_decl) => {
                if let Some(dep) = deps.get(raw_decl.typ.as_str()) {
                    let typ = Rc::clone(dep);
                    let typ: TopDeclType = if let Some(item_size) = dep.total_size() {
                        FixedVector { item_size, typ }.into()
                    } else {
                        DynamicVector { typ }.into()
                    };
                    Some(TopDecl::new(raw.name(), typ))
                } else {
                    None
                }
            }
            RawTopDecl::Table(raw_decl) => {
                let mut inner = Vec::with_capacity(raw_decl.inner.len());
                let mut offset = 4 + 4 * raw_decl.inner.len();
                let mut offsets = Vec::new();
                let mut body = Vec::new();
                for raw_field in &raw_decl.inner[..] {
                    let field_name = raw_field.name.to_owned();
                    if let Some(dep) = deps.get(raw_field.typ.as_str()) {
                        let field = FieldDecl {
                            name: field_name,
                            typ: Rc::clone(dep),
                        };
                        offsets.extend_from_slice(&(offset as u32).to_le_bytes()[..]);
                        let field_content = field.typ.default_content();
                        body.extend_from_slice(&field_content[..]);
                        offset += field_content.len();
                        inner.push(field);
                    } else {
                        break;
                    }
                }
                if inner.len() == raw_decl.inner.len() {
                    let mut content = Vec::new();
                    content.extend_from_slice(&(offset as u32).to_le_bytes()[..]);
                    content.extend_from_slice(&offsets[..]);
                    content.extend_from_slice(&body[..]);
                    assert_eq!(content.len(), offset);
                    let typ: TopDeclType = Table { content, inner }.into();
                    Some(TopDecl::new(raw.name(), typ))
                } else {
                    None
                }
            }
        }
    }
}

impl Ast {
    pub(crate) fn new(raw: RawAst) -> Self {
        let mut decls_idx = HashMap::new();
        let mut decls_keys = HashSet::new();
        for decl in &raw.decls[..] {
            let name = decl.name();
            if name == ATOM_NAME {
                panic!("the name `{}` is reserved", name);
            }
            if decls_idx.insert(name, decl).is_some() || !decls_keys.insert(name) {
                panic!("the name `{}` is used more than once", name);
            };
        }
        let mut decls_result = HashMap::new();
        decls_result.insert(ATOM_NAME, Rc::new(TopDecl::atom()));
        loop {
            if decls_keys.is_empty() {
                break;
            }
            let incompleted = decls_keys.len();
            decls_keys.retain(|&name| {
                let decl_raw = decls_idx.get(name).unwrap();
                if let Some(decl) = TopDecl::complete(decl_raw, &decls_result) {
                    decls_result.insert(name, Rc::new(decl));
                    false
                } else {
                    true
                }
            });
            if decls_keys.len() == incompleted {
                panic!(
                    "there are {} types which are unable to be completed: {:?}",
                    incompleted, decls_keys
                );
            }
        }
        let mut decls = Vec::with_capacity(raw.decls.len());
        for decl in &raw.decls[..] {
            let result = decls_result.get(decl.name()).unwrap();
            decls.push(Rc::clone(result));
        }
        Self { decls }
    }
}
