use core::mem::MaybeUninit;

/// Inline HashMap
pub mod hashmap;
/// Inline vector
pub mod vec;

fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    [const { MaybeUninit::uninit() }; N]
}

#[cfg(test)]
mod bounded_tests;
