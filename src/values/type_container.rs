use std::cell::RefCell;
use std::rc::Rc;
use crate::values::core_values::r#type::r#type::Type;
use crate::values::type_reference::TypeReference;


// TODO: move match logic and other type stuff here
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeContainer {
    Type(Type),
    TypeReference(Rc<RefCell<TypeReference>>),
}

/**

ValueContainer           <----    TypeContainer

  Value
     Type                <----
     ... 
  Reference
     ValueReference
     TypeReference       <-----

*/

impl TypeContainer {
    
}