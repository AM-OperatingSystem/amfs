use bitvec::prelude::*;
use std::collections::BTreeSet;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

macro_rules! set {
    ( $( $x:expr ),* ) => {  // Match zero or more comma delimited items
        {
            let mut temp_set = BTreeSet::new();  // Create a mutable BTreeSet
            $(
                temp_set.insert($x); // Insert each item matched into the BTreeSet
            )*
            temp_set // Return the populated BTreeSet
        }
    };
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, EnumIter, Debug)]
/// The set of defined AMFS features
pub enum AMFeatures {
    /// The base feature, always true
    Base,
    /// The never feature, always false
    Never,
}

impl AMFeatures {
    /// Returns the feature map for the current AMFS version
    #[cfg(feature = "unstable")]
    pub fn current() -> BitArr!(for 2048) {
        let mut res = bitarr![0; 2048];
        for i in AMFeatures::current_set() {
            res.set(i, true);
        }
        res
    }
    /// Returns a vector of features for the current AMFS version
    #[cfg(feature = "unstable")]
    pub fn current_set() -> BTreeSet<usize> {
        set![AMFeatures::Base]
            .iter()
            .map(|x| *x as usize)
            .collect::<BTreeSet<usize>>()
    }
    /// Converts a bit array to a set of features
    #[cfg(feature = "stable")]
    pub fn bit2set(map: &BitArr!(for 2048)) -> BTreeSet<AMFeatures> {
        let mut res = BTreeSet::new();
        for i in AMFeatures::iter() {
            if *map.get(i as usize).unwrap() {
                res.insert(i);
            }
        }
        res
    }
}
