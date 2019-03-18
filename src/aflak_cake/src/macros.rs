use std::collections::BTreeMap;
use std::ops;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

use boow::Bow;

use super::{ConvertibleVariants, TransformInputSlot, TypeId, VariantName, DST};
use compute::ComputeError;
use export::{DeserDST, ImportError, NamedAlgorithms};

pub struct MacroHandle<'t, T: 't, E: 't> {
    inner: Arc<RwLock<Macro<'t, T, E>>>,
}

impl<'t, T, E> Clone for MacroHandle<'t, T, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct Macro<'t, T: 't, E: 't> {
    id: usize,
    dst: DST<'t, T, E>,
    updated_on: Instant,
}

impl<'t, T, E> Clone for Macro<'t, T, E>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            dst: self.dst.clone(),
            updated_on: self.updated_on,
        }
    }
}

/// TODO
impl<'t, T, E> MacroHandle<'t, T, E> {
    pub fn name(&self) -> String {
        "Macro".to_owned()
    }

    pub fn input_types(&self) -> Vec<TypeId> {
        vec![]
    }

    pub fn outputs(&self) -> Vec<TypeId> {
        vec![]
    }

    pub fn inputs(&self) -> Vec<TransformInputSlot<T>> {
        vec![]
    }

    pub fn defaults(&self) -> Vec<Option<T>> {
        vec![]
    }

    pub fn id(&self) -> usize {
        self.read().id
    }

    pub fn call(&self, _args: Vec<Bow<'_, T>>) -> Vec<Result<T, Arc<ComputeError<E>>>> {
        vec![]
    }

    pub fn get_macro(&self) -> Macro<'t, T, E>
    where
        T: Clone,
    {
        self.read().clone()
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Macro<'t, T, E>> {
        self.inner.read().unwrap()
    }

    pub fn write(&self) -> MacroMut<'_, 't, T, E> {
        MacroMut {
            inner: self.inner.write().unwrap(),
            changed: false,
        }
    }

    pub fn updated_on(&self) -> Instant {
        self.read().updated_on
    }
}

impl<'t, T, E> Macro<'t, T, E> {
    pub fn dst(&self) -> &DST<'t, T, E> {
        &self.dst
    }

    pub fn dst_mut(&mut self) -> &mut DST<'t, T, E> {
        &mut self.dst
    }
}

pub struct MacroMut<'a, 't: 'a, T: 't, E: 't> {
    inner: RwLockWriteGuard<'a, Macro<'t, T, E>>,
    changed: bool,
}

impl<'a, 't, T, E> Drop for MacroMut<'a, 't, T, E> {
    fn drop(&mut self) {
        if self.changed {
            self.inner.updated_on = Instant::now();
        }
    }
}

impl<'a, 't, T, E> ops::Deref for MacroMut<'a, 't, T, E> {
    type Target = Macro<'t, T, E>;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, 't, T, E> ops::DerefMut for MacroMut<'a, 't, T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        self.inner.deref_mut()
    }
}

pub struct MacroManager<'t, T: 't, E: 't> {
    cnt: usize,
    macros: BTreeMap<usize, MacroHandle<'t, T, E>>,
}

impl<'t, T, E> MacroManager<'t, T, E> {
    pub fn get_macro(&self, id: usize) -> Option<&MacroHandle<'t, T, E>> {
        self.macros.get(&id)
    }

    pub fn new() -> Self {
        MacroManager {
            cnt: 0,
            macros: BTreeMap::new(),
        }
    }

    pub fn create_macro(&mut self) -> &MacroHandle<'t, T, E> {
        self.cnt += 1;
        self.macros.insert(
            self.cnt,
            MacroHandle {
                inner: Arc::new(RwLock::new(Macro {
                    id: self.cnt,
                    dst: DST::new(),
                    updated_on: Instant::now(),
                })),
            },
        );
        self.macros.get(&self.cnt).unwrap()
    }

    pub fn to_serializable(&self) -> SerdeMacroManager<T>
    where
        T: Clone + VariantName,
    {
        SerdeMacroManager::from(self)
    }
}

impl<T, E> MacroManager<'static, T, E> {
    pub fn from_deserializable(&mut self, deser: SerdeMacroManager<T>) -> Result<(), ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        deser
            .into_macro_manager(&MacroManager::new())
            .map(|new_manager| {
                *self = new_manager;
            })
    }
}

impl<'a, 't, T, E> From<&'a MacroManager<'t, T, E>> for SerdeMacroManager<T>
where
    T: Clone + VariantName,
{
    fn from(manager: &'a MacroManager<'t, T, E>) -> Self {
        Self {
            macros: manager
                .macros
                .values()
                .map(|handle| SerdeMacro::from(&*handle.read()))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerdeMacro<T> {
    id: usize,
    dst: DeserDST<T>,
}

impl<'t, 'd, T, E> From<&'d Macro<'t, T, E>> for SerdeMacro<T>
where
    T: Clone + VariantName,
{
    fn from(macr: &'d Macro<'t, T, E>) -> Self {
        Self {
            id: macr.id,
            dst: DeserDST::from_dst(&macr.dst),
        }
    }
}

impl<T> SerdeMacro<T> {
    fn into_macro<E>(
        self,
        macro_manager: &MacroManager<'static, T, E>,
    ) -> Result<Macro<'static, T, E>, ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        // TODO: Deal with nested macros
        let id = self.id;
        self.dst.into_dst(macro_manager).map(move |dst| Macro {
            id,
            dst,
            updated_on: Instant::now(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerdeMacroManager<T> {
    macros: Vec<SerdeMacro<T>>,
}

impl<T> SerdeMacroManager<T> {
    fn into_macro_manager<E>(
        self,
        macro_manager: &MacroManager<'static, T, E>,
    ) -> Result<MacroManager<'static, T, E>, ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        let cnt = self.macros.iter().map(|macr| macr.id).max().unwrap_or(0);
        let mut macros = BTreeMap::new();
        for macr in self.macros {
            let macr = macr.into_macro(macro_manager)?;
            macros.insert(
                macr.id,
                MacroHandle {
                    inner: Arc::new(RwLock::new(macr)),
                },
            );
        }
        Ok(MacroManager { cnt, macros })
    }
}
