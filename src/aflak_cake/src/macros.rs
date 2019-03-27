use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::ops;
use std::ptr;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

use boow::Bow;
use uuid::Uuid;

use super::{
    Algorithm, ConvertibleVariants, InputSlot, Output, Transform, TransformInputSlot, TypeId,
    VariantName, DST,
};
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

impl<'t, T, E> PartialEq for MacroHandle<'t, T, E> {
    fn eq(&self, other: &MacroHandle<'t, T, E>) -> bool {
        ptr::eq(self.inner.as_ref(), other.inner.as_ref())
    }
}

impl<'t, T, E> fmt::Debug for MacroHandle<'t, T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MacroHandle")
            .field("inner", &format!("{:p}", &*self.inner))
            .field("id", &self.id().to_hyphenated().to_string())
            .field("name", &self.name())
            .finish()
    }
}

pub struct Macro<'t, T: 't, E: 't> {
    id: Uuid,
    name: String,
    inputs: Vec<MacroInput<T>>,
    dst: DST<'t, T, E>,
    updated_on: Instant,
}

#[derive(Clone)]
struct MacroInput<T> {
    name: &'static str,
    slot: InputSlot,
    type_id: Option<TypeId>,
    default: Option<T>,
}

impl<'t, T, E> Clone for Macro<'t, T, E>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            inputs: self.inputs.clone(),
            dst: self.dst.clone(),
            updated_on: self.updated_on,
        }
    }
}

impl<'t, T, E> From<Macro<'t, T, E>> for MacroHandle<'t, T, E> {
    fn from(macr: Macro<'t, T, E>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(macr)),
        }
    }
}

impl<'t, T, E> MacroHandle<'t, T, E> {
    pub fn name(&self) -> String {
        self.read().name.clone()
    }

    pub fn name_mut(&self) -> MacroNameMut<'_, 't, T, E> {
        MacroNameMut {
            guard: self.inner.write().unwrap(),
        }
    }

    pub fn input_types(&self) -> Vec<TypeId> {
        self.read().input_types()
    }

    pub fn outputs(&self) -> Vec<TypeId>
    where
        T: VariantName,
    {
        self.read().outputs()
    }

    pub fn inputs(&self) -> Vec<TransformInputSlot<T>>
    where
        T: Clone,
    {
        self.read().inputs()
    }

    pub fn defaults(&self) -> Vec<Option<T>>
    where
        T: Clone,
    {
        self.read().defaults()
    }

    pub fn id(&self) -> Uuid {
        self.read().id
    }

    pub fn call(&self, args: Vec<Bow<'_, T>>) -> Vec<Result<T, Arc<ComputeError<E>>>>
    where
        T: Clone + VariantName + ConvertibleVariants,
    {
        self.read().call(args)
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Macro<'t, T, E>> {
        self.inner.read().unwrap()
    }

    pub fn write(&self) -> MacroMut<'_, 't, T, E>
    where
        T: Clone + VariantName,
    {
        MacroMut {
            inner: self.inner.write().unwrap(),
            changed: false,
        }
    }

    pub fn updated_on(&self) -> Instant {
        self.read().updated_on
    }

    /// Get all children, ordered from deepest to shallowest
    pub fn children_deep(&self) -> impl Iterator<Item = MacroHandle<'t, T, E>> {
        fn inner<'t, T, E>(
            macr: &MacroHandle<'t, T, E>,
            mut already_selected: Vec<MacroHandle<'t, T, E>>,
        ) -> Vec<MacroHandle<'t, T, E>> {
            let lock = macr.read();
            if already_selected.contains(macr) {
                already_selected
            } else {
                already_selected.push(macr.clone());
                for child in lock.children_shallow() {
                    already_selected = inner(child, already_selected);
                }
                already_selected
            }
        }

        let mut out = inner(self, vec![]);
        if let Some(pos) = out.iter().position(|handle| handle == self) {
            out.remove(pos);
        }
        out.reverse();
        out.into_iter()
    }
}

impl<'t, T, E> Macro<'t, T, E> {
    pub fn dst(&self) -> &DST<'t, T, E> {
        &self.dst
    }

    pub fn dst_mut(&mut self) -> &mut DST<'t, T, E> {
        &mut self.dst
    }

    fn input_types(&self) -> Vec<TypeId> {
        self.inputs
            .iter()
            .map(|input| input.type_id.unwrap_or(TypeId("No type")))
            .collect()
    }

    fn inputs(&self) -> Vec<TransformInputSlot<T>>
    where
        T: Clone,
    {
        self.inputs
            .iter()
            .map(|input| TransformInputSlot {
                type_id: input.type_id.unwrap_or(TypeId("No type")),
                default: input.default.clone(),
                name: input.name,
            })
            .collect()
    }

    fn outputs(&self) -> Vec<TypeId>
    where
        T: VariantName,
    {
        self.dst
            .outputs_iter()
            .map(|(_, some_output)| {
                if let Some(output) = some_output {
                    let t = self.dst.get_transform(output.t_idx).unwrap();
                    t.outputs()[output.index()]
                } else {
                    TypeId("No type")
                }
            })
            .collect()
    }

    fn defaults(&self) -> Vec<Option<T>>
    where
        T: Clone,
    {
        self.inputs
            .iter()
            .map(|input| &input.default)
            .cloned()
            .collect()
    }

    fn find_default_inputs(dst: &DST<'t, T, E>) -> Vec<MacroInput<T>>
    where
        T: Clone + VariantName,
    {
        let mut inputs = vec![];
        for input_slot in dst.unattached_input_slots() {
            let input = match input_slot {
                InputSlot::Output(_) => MacroInput {
                    name: "Out",
                    slot: input_slot,
                    type_id: None,
                    default: None,
                },
                InputSlot::Transform(input) => {
                    let defaults = dst.get_default_inputs(input.t_idx).expect("Get transform");
                    let t_inputs = dst
                        .get_transform(input.t_idx)
                        .expect("Get transform")
                        .inputs();
                    let default = &defaults[input.index()];
                    let t_input = &t_inputs[input.index()];
                    MacroInput {
                        name: t_input.name,
                        slot: input_slot,
                        type_id: Some(t_input.type_id),
                        default: default.clone(),
                    }
                }
            };
            inputs.push(input);
        }
        inputs
    }

    fn call(&self, args: Vec<Bow<'_, T>>) -> Vec<Result<T, Arc<ComputeError<E>>>>
    where
        T: Clone + VariantName + ConvertibleVariants,
    {
        let mut cache = HashMap::new();

        let mut dst = self.dst.clone();

        for (arg, input) in args.into_iter().zip(&self.inputs) {
            let t_idx = dst.add_owned_transform(Transform::new_constant((*arg).clone()));
            let output = Output::new(t_idx, 0);
            match input.slot {
                InputSlot::Output(output_id) => dst.update_output(output_id, output),
                InputSlot::Transform(input) => {
                    if let Err(e) = dst.connect(output, input) {
                        unimplemented!("Type error: {}", e);
                    }
                }
            }
        }

        dst.outputs_iter()
            .map(|(output_id, _)| dst.compute_sync(*output_id, &mut cache))
            .collect()
    }

    fn children_shallow(&self) -> impl Iterator<Item = &MacroHandle<'t, T, E>> {
        self.dst()
            .transforms_iter()
            .filter_map(|(_, t)| match t.algorithm() {
                Algorithm::Macro { handle } => Some(handle),
                _ => None,
            })
    }
}

pub struct MacroMut<'a, 't: 'a, T: 't + Clone + VariantName, E: 't> {
    inner: RwLockWriteGuard<'a, Macro<'t, T, E>>,
    changed: bool,
}

impl<'a, 't, T: Clone + VariantName, E> Drop for MacroMut<'a, 't, T, E> {
    fn drop(&mut self) {
        if self.changed {
            self.inner.updated_on = Instant::now();
            self.inner.inputs = Macro::find_default_inputs(&self.inner.dst);
        }
    }
}

impl<'a, 't, T: Clone + VariantName, E> ops::Deref for MacroMut<'a, 't, T, E> {
    type Target = Macro<'t, T, E>;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, 't, T: Clone + VariantName, E> ops::DerefMut for MacroMut<'a, 't, T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        self.inner.deref_mut()
    }
}

pub struct MacroNameMut<'a, 't: 'a, T: 't, E: 't> {
    guard: RwLockWriteGuard<'a, Macro<'t, T, E>>,
}

impl<'a, 't, T, E> ops::Deref for MacroNameMut<'a, 't, T, E> {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.guard.name
    }
}

impl<'a, 't, T, E> ops::DerefMut for MacroNameMut<'a, 't, T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard.name
    }
}

pub struct MacroManager<'t, T: 't, E: 't> {
    macros: BTreeMap<Uuid, MacroHandle<'t, T, E>>,
}

impl<'t, T, E> fmt::Debug for MacroManager<'t, T, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MacroManager")
            .field("macros", &self.macros)
            .finish()
    }
}

impl<'t, T, E> MacroManager<'t, T, E> {
    pub fn get_macro(&self, id: Uuid) -> Option<&MacroHandle<'t, T, E>> {
        self.macros.get(&id)
    }

    pub fn new() -> Self {
        MacroManager {
            macros: BTreeMap::new(),
        }
    }

    pub fn create_macro(&mut self) -> &MacroHandle<'t, T, E>
    where
        T: Clone + VariantName,
    {
        let dst = DST::new();
        let id = Uuid::new_v4();
        // FIXME: Reading from all macros may cause deadlock if the caller is
        // currently writing to a macor.
        let name = (1..)
            .map(|cnt| format!("New macro #{}", cnt))
            .find(|name| self.macros.values().all(|macr| &macr.name() != name))
            .unwrap();
        self.macros.insert(
            id,
            MacroHandle::from(Macro {
                id,
                name,
                inputs: Macro::find_default_inputs(&dst),
                dst,
                updated_on: Instant::now(),
            }),
        );
        self.macros.get(&id).unwrap()
    }

    pub fn to_serializable(&self) -> SerdeMacroManager<T>
    where
        T: Clone + VariantName,
    {
        SerdeMacroManager::from(self)
    }

    pub fn macros(&self) -> impl Iterator<Item = &MacroHandle<'t, T, E>> {
        self.macros.values()
    }

    /// Used internally for import/export.
    /// Fail if a macro with the same ID is already added.
    pub fn add_macro(&mut self, macr: Macro<'t, T, E>) -> Result<(), ImportError> {
        if self.macros.contains_key(&macr.id) {
            Err(ImportError::DuplicateMacroId(macr.id))
        } else {
            self.macros.insert(macr.id, MacroHandle::from(macr));
            Ok(())
        }
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
    id: Uuid,
    name: String,
    dst: DeserDST<T>,
}

impl<'t, 'd, T, E> From<&'d Macro<'t, T, E>> for SerdeMacro<T>
where
    T: Clone + VariantName,
{
    fn from(macr: &'d Macro<'t, T, E>) -> Self {
        Self {
            id: macr.id,
            name: macr.name.clone(),
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
        let name = self.name;
        self.dst.into_dst(macro_manager).map(move |dst| Macro {
            id,
            name,
            inputs: Macro::find_default_inputs(&dst),
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
        let mut macros = BTreeMap::new();
        for macr in self.macros {
            let macr = macr.into_macro(macro_manager)?;
            macros.insert(macr.id, MacroHandle::from(macr));
        }
        Ok(MacroManager { macros })
    }
}

/// Stand-along format to save macro along with its dependency sub-macros
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerdeMacroStandAlone<T> {
    main: SerdeMacro<T>,
    subs: Vec<SerdeMacro<T>>,
}

impl<T> SerdeMacroStandAlone<T> {
    pub fn into_macro<E>(self) -> Result<Macro<'static, T, E>, ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        let mut macro_manager = MacroManager::new();
        for macr in self.subs {
            let sub = macr.into_macro(&macro_manager)?;
            macro_manager.add_macro(sub)?;
        }
        self.main.into_macro(&macro_manager)
    }
}

impl<'t, 'd, T, E> From<&'d MacroHandle<'t, T, E>> for SerdeMacroStandAlone<T>
where
    T: Clone + VariantName,
{
    fn from(handle: &'d MacroHandle<'t, T, E>) -> Self {
        let main = SerdeMacro::from(&*handle.read());
        Self {
            main,
            subs: handle
                .children_deep()
                .map(|handle| SerdeMacro::from(&*handle.read()))
                .collect(),
        }
    }
}

/// Stand-alone format to a full DST along with its dependency sub-macros
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerdeDSTStandAlone<T> {
    main: DeserDST<T>,
    subs: Vec<SerdeMacro<T>>,
}

impl<T> SerdeDSTStandAlone<T> {
    pub fn into_dst<E>(
        self,
    ) -> Result<(DST<'static, T, E>, MacroManager<'static, T, E>), ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        let mut macro_manager = MacroManager::new();
        for macr in self.subs {
            let sub = macr.into_macro(&macro_manager)?;
            macro_manager.add_macro(sub)?;
        }
        self.main
            .into_dst(&macro_manager)
            .map(|dst| (dst, macro_manager))
    }
}

impl<'t, 'd, T, E> From<&'d DST<'t, T, E>> for SerdeDSTStandAlone<T>
where
    T: Clone + VariantName,
{
    fn from(dst: &'d DST<'t, T, E>) -> Self {
        let tmp = MacroHandle::from(Macro {
            id: Uuid::nil(),
            name: String::new(),
            inputs: Macro::find_default_inputs(dst),
            dst: dst.clone(),
            updated_on: Instant::now(),
        });
        Self {
            main: DeserDST::from_dst(dst),
            subs: tmp
                .children_deep()
                .map(|handle| SerdeMacro::from(&*handle.read()))
                .collect(),
        }
    }
}
