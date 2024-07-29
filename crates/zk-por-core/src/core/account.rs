use crate::types::F;

/// A struct representing a users account. It represents their assets and debt as a Vector of goldilocks field elements. 
#[derive(Debug, Clone)]
pub struct Account{
    pub id: String,
    pub assets: Vec<F>,
    pub debt: Vec<F>
}