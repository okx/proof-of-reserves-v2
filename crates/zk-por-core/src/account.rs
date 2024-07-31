use crate::types::F;

/// A struct representing a users account. It represents their equity and debt as a Vector of goldilocks field elements.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub equity: Vec<F>,
    pub debt: Vec<F>,
}
