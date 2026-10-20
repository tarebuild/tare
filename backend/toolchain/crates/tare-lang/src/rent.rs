//! Exact fast paths for the SVM rent formula used by generated constraints.
use solana_program::rent::Rent;

#[inline(always)]
pub fn minimum_balance(rent: &Rent, data_len: usize) -> u64 {
    // The SDK converts the product to f64 before multiplying. Integer shortcuts
    // are equivalent only while that conversion is exact. Overflow and all
    // other thresholds remain in the SDK, including its panic/cast behavior.
    if let Some(bytes) = (data_len as u64).checked_add(128) {
        let rate = rent.lamports_per_byte_year;
        // A division guard avoids LLVM lowering checked_mul to a software
        // 128-bit multiply on sBPF, while proving the same exact-f64 bound.
        // The guarded product cannot overflow, even with overflow checks enabled.
        if rate == 0 || bytes <= (1u64 << 53) / rate {
            let product = bytes.wrapping_mul(rate);
            match rent.exemption_threshold.to_bits() {
                0x3ff0_0000_0000_0000 => return product,
                0x4000_0000_0000_0000 => return product.wrapping_mul(2),
                _ => {}
            }
        }
    }
    rent.minimum_balance(data_len)
}

#[inline(always)]
pub fn is_exempt(rent: &Rent, balance: u64, data_len: usize) -> bool {
    balance >= minimum_balance(rent, data_len)
}

#[cfg(target_os = "solana")]
extern "C" {
    fn sol_get_rent_sysvar(address: *mut u8) -> u64;
}

/// Read the dedicated Rent syscall into its output storage. The runtime writes
/// the complete Rust Rent value; unused padding is never read by this program.
#[inline(always)]
pub fn get() -> Result<Rent, solana_program::program_error::ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let mut output=std::mem::MaybeUninit::<Rent>::uninit();
        let status=unsafe {sol_get_rent_sysvar(output.as_mut_ptr().cast())};
        if status==0 {Ok(unsafe {output.assume_init()})}
        else {Err(solana_program::program_error::ProgramError::from(status))}
    }
    #[cfg(not(target_os = "solana"))]
    {
        use solana_program::sysvar::Sysvar;
        Rent::get()
    }
}
