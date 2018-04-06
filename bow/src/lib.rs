use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

#[derive(Copy, Clone)]
pub enum Bow<'a, T: 'a> {
    Owned(T),
    Borrowed(&'a T),
}

impl<'a, T: 'a> Borrow<T> for Bow<'a, T> {
    fn borrow(&self) -> &T {
        match self {
            &Bow::Owned(ref t) => t,
            &Bow::Borrowed(t) => t,
        }
    }
}

impl<'a, T: 'a> Deref for Bow<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.borrow()
    }
}

impl<'a, T: 'a> Bow<'a, T> {
    pub fn borrow_mut(&mut self) -> Option<&mut T> {
        match self {
            &mut Bow::Owned(ref mut t) => Some(t),
            &mut Bow::Borrowed(_) => None,
        }
    }

    pub fn extract(self) -> Option<T> {
        match self {
            Bow::Owned(t) => Some(t),
            Bow::Borrowed(_) => None,
        }
    }
}

impl<'a, T: 'a> Eq for Bow<'a, T>
where
    T: Eq,
{
}

impl<'a, T: 'a> Ord for Bow<'a, T>
where
    T: Ord,
{
    fn cmp(&self, other: &Bow<'a, T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<'a, T: 'a> PartialEq for Bow<'a, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Bow<'a, T>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<'a, T: 'a> PartialOrd for Bow<'a, T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Bow<'a, T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<'a, T: 'a> fmt::Debug for Bow<'a, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: 'a> fmt::Display for Bow<'a, T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: 'a> Default for Bow<'a, T>
where
    T: Default,
{
    fn default() -> Self {
        Bow::Owned(T::default())
    }
}

impl<'a, T: 'a> Hash for Bow<'a, T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<'a, T: 'a> AsRef<T> for Bow<'a, T> {
    fn as_ref(&self) -> &T {
        self
    }
}
