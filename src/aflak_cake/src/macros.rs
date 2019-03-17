use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use boow::Bow;

use super::{TransformInputSlot, TypeId};
use compute::ComputeError;

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

#[derive(Serialize, Deserialize)]
pub struct Macro<'t, T: 't, E: 't> {
    id: usize,
    _mark: PhantomData<(&'t PhantomData<T>, E)>,
}

impl<'t, T, E> Clone for Macro<'t, T, E> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _mark: PhantomData,
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

    pub fn get_macro(&self) -> Macro<'t, T, E> {
        self.read().clone()
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Macro<'t, T, E>> {
        self.inner.read().unwrap()
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
                    _mark: PhantomData,
                })),
            },
        );
        self.macros.get(&self.cnt).unwrap()
    }

    pub fn to_serializable(&self) -> SerdeMacroManager {
        SerdeMacroManager::from(self)
    }

    pub fn from_deserializable(&mut self, deser: SerdeMacroManager) {
        *self = Self::from(deser);
    }
}

impl<'t, T, E> From<SerdeMacroManager> for MacroManager<'t, T, E> {
    fn from(deser: SerdeMacroManager) -> MacroManager<'t, T, E> {
        let cnt = deser.macros.iter().map(|macr| macr.id).max().unwrap_or(0);
        let mut macros = BTreeMap::new();
        for macr in deser.macros {
            macros.insert(
                macr.id,
                MacroHandle {
                    inner: Arc::new(RwLock::new(Macro::from(macr))),
                },
            );
        }
        MacroManager { cnt, macros }
    }
}

impl<'a, 't, T, E> From<&'a MacroManager<'t, T, E>> for SerdeMacroManager {
    fn from(manager: &'a MacroManager<'t, T, E>) -> SerdeMacroManager {
        SerdeMacroManager {
            macros: manager
                .macros
                .values()
                .map(|handle| SerdeMacro::from(handle.get_macro()))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerdeMacro {
    id: usize,
}

impl<'t, T, E> From<Macro<'t, T, E>> for SerdeMacro {
    fn from(macr: Macro<'t, T, E>) -> Self {
        Self { id: macr.id }
    }
}

impl<'t, T, E> From<SerdeMacro> for Macro<'t, T, E> {
    fn from(macr: SerdeMacro) -> Self {
        Self {
            id: macr.id,
            _mark: PhantomData,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerdeMacroManager {
    macros: Vec<SerdeMacro>,
}
