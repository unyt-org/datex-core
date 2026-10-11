use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Generics, Type};

#[derive(Debug, PartialEq, Eq)]
pub enum TypeKind {
    Entity,
    Structural { only_structural: bool },
}

impl TypeKind {
    pub fn is_entity(&self) -> bool {
        matches!(self, TypeKind::Entity)
    }
    pub fn is_structural(&self) -> bool {
        matches!(self, TypeKind::Structural { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Namespace {
    None,
    Module,
    Named(String),
}

#[derive(Debug, PartialEq, Default)]
pub enum FieldMapping {
    /// The field contains a value that implements [DatexNative] and other traits and can be mapped directly to and from DATEX
    #[default]
    Datex,
    /// The field derives serde Serialize and/or Deserialize and can be mapped to and from DATEX using Serde
    Serde,
}

impl FieldMapping {
    pub fn is_datex(&self) -> bool {
        matches!(self, FieldMapping::Datex)
    }
    pub fn is_serde(&self) -> bool {
        matches!(self, FieldMapping::Serde)
    }
}

/// Top-level attributes for the Datex derive macro
#[derive(Debug, PartialEq)]
pub struct StructureAttributes {
    /// Internally used attribute to indicate that the macro should use the `datex_core` namespace
    /// instead of inferring it. This is required for doctests to work.
    pub force_datex_core_namespace: bool,

    /// Optional override for the exported name of the type. Defaults to the Rust struct or enum name.
    pub datex_name: Option<String>,

    /// If the decorated struct or enum should not be deserializable from a Datex value.
    pub no_deserialize: bool,

    /// When set to true, the struct/enum will map to a DATEX structural type instead of a nominal entity type.
    pub type_kind: TypeKind,

    /// If the decorated struct or enum should be exported to the Datex registry.
    /// `#[datex(export)]`
    pub export: bool,
    pub docs: Option<String>,
}

pub trait FieldIdent {
    /// Returns a normalized identifier for the field that can be used as a variable name.
    /// For named fields, it returns the field name as an identifier.
    /// For unnamed fields, it returns identifiers like `_0`, `_1`, etc.
    fn normalized_ident(&self) -> Ident;
    /// Returns the original name of the field as defined in the Rust struct.
    fn original_name(&self) -> String;
    /// The actual field accessor of the rust struct, e.g. `field_name` for named fields, or `0`, `1`, etc. for unnamed fields.
    fn accessor(&self) -> TokenStream;
}

impl FieldIdent for NamedField {
    fn normalized_ident(&self) -> Ident {
        self.ident_accessor()
    }
    fn original_name(&self) -> String {
        self.name.clone()
    }
    fn accessor(&self) -> TokenStream {
        self.ident_accessor().into_token_stream()
    }
}

impl FieldIdent for IndexedField {
    fn normalized_ident(&self) -> Ident {
        Ident::new(&format!("_{}", self.index), Span::call_site())
    }
    fn original_name(&self) -> String {
        format!("{}", self.index)
    }
    fn accessor(&self) -> TokenStream {
        self.index_accessor().into_token_stream()
    }
}

pub trait FieldType {
    fn ty(&self) -> &Type;
}

impl FieldType for Field {
    fn ty(&self) -> &Type {
        &self.ty
    }
}

impl FieldType for NamedField {
    fn ty(&self) -> &Type {
        self.field.ty()
    }
}

impl FieldType for IndexedField {
    fn ty(&self) -> &Type {
        self.field.ty()
    }
}

pub trait AnyField: FieldIdent + FieldType {}
impl<T> AnyField for T where T: FieldIdent + FieldType {}

#[derive(Debug, PartialEq)]
/// Represents a field in a struct or enum variant, along with its type and attributes.
pub struct Field {
    pub ty: Type,
    pub attributes: FieldAttributes,
}

#[derive(Debug, PartialEq)]
/// Represents a field in a struct or enum variant, along with its type and attributes.
pub struct IndexedField {
    pub index: usize,
    pub field: Field,
}

impl IndexedField {
    pub fn index_accessor(&self) -> syn::Index {
        syn::Index::from(self.index)
    }
}

#[derive(Debug, PartialEq)]
/// General attributes that can be applied to any field.
pub struct FieldAttributes {
    pub field_mapping: FieldMapping,
}

#[derive(Debug, PartialEq)]
/// Attributes specific to named fields in structs or enum variants.
pub struct NamedFieldAttributes {
    /// Skip the field for DATEX representation, but allow deserialization from DATEX by using the default value for the field.
    pub skip_with_default: bool,
    /// An optional rename for the field used for the DATEX representation. If not provided, the rust field name will be used.
    pub rename: Option<String>,
}

#[derive(Debug, PartialEq)]
/// Represents a named field in a struct or enum variant.
pub struct NamedField {
    pub name: String,
    pub field: Field,
    pub attributes: NamedFieldAttributes,
}

impl NamedField {
    /// Returns the name of the field to be used in the DATEX representation.
    /// This will return the `rename` attribute if it is set, otherwise it will return the original Rust field name.
    pub fn datex_field_name(&self) -> &str {
        self.attributes
            .rename
            .as_deref()
            .unwrap_or(self.name.as_str())
    }
    pub fn ident_accessor(&self) -> Ident {
        Ident::new(&self.name, Span::call_site())
    }
}

#[derive(Debug, PartialEq)]
/// Represents the different kinds of fields a struct or enum variant can have.
pub enum Fields {
    Named(Vec<NamedField>),
    Unnamed(Vec<IndexedField>),
    Transparent(IndexedField),
    Unit,
}

impl Fields {
    /// Returns a vector of normalized identifiers for the fields
    /// that can be used as variable names.
    /// For named fields, it returns the field names as identifiers.
    /// For unnamed fields, it returns identifiers like `_0`, `_1`, etc.
    pub fn normalized_field_idents(&self) -> Vec<Ident> {
        self.iter().map(|f| f.normalized_ident()).collect()
    }

    /// Returns a list of all field accessors (field names or indices) as TokenStreams.
    pub fn field_accessors(&self) -> Vec<TokenStream> {
        self.iter().map(|f| f.accessor()).collect()
    }

    pub fn is_named(&self) -> bool {
        matches!(self, Fields::Named(_))
    }

    /// Returns an iterator over all fields as `&dyn AnyField`.
    pub gen fn iter(&self) -> &dyn AnyField {
        match self {
            Fields::Named(fields) => {
                for field in fields {
                    yield field as &dyn AnyField;
                }
            }
            Fields::Unnamed(fields) => {
                for field in fields {
                    yield field as &dyn AnyField;
                }
            }
            Fields::Transparent(field) => yield field as &dyn AnyField,
            Fields::Unit => {}
        }
    }

    pub fn wrap_in_initializer(
        &self,
        inner: impl Iterator<Item = TokenStream>,
    ) -> TokenStream {
        match self {
            Fields::Named(fields) => {
                let field_initializers =
                    fields.iter().zip(inner).map(|(field, initializer)| {
                        let field_name = field.ident_accessor();
                        quote! { #field_name: #initializer }
                    });
                quote! { { #(#field_initializers),* } }
            }
            Fields::Unnamed(fields) => {
                let field_initializers =
                    fields.iter().zip(inner).map(|(_field, initializer)| {
                        quote! { #initializer }
                    });
                quote! { ( #(#field_initializers),* ) }
            }
            Fields::Transparent(_field) => {
                let initializer = inner.into_iter().next().expect(
                    "Transparent field should have exactly one initializer",
                );
                quote! { (#initializer) }
            }
            Fields::Unit => {
                quote! {}
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Fields,
    // TODO: enum variant attributes?
}
impl EnumVariant {
    pub fn ident(&self) -> Ident {
        Ident::new(&self.name, Span::call_site())
    }
}

#[derive(Debug, PartialEq)]
pub enum Structure {
    Enum(Vec<EnumVariant>),
    Struct(Fields),
}

#[derive(Debug, PartialEq)]
pub struct StructureData {
    pub namespace: Vec<String>,
    pub ident: Ident,
    pub generics: Generics,
    pub attributes: StructureAttributes,
    pub structure: Structure,
}
