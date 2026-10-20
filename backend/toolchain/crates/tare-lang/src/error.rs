use crate::{error_code, Pubkey};
use solana_program::program_error::ProgramError;
use std::fmt::{self, Write};
#[derive(Debug, Clone, Copy)]
pub struct Source {
    pub filename: &'static str,
    pub line: u32,
}
// An explicit tag keeps clone/drop dispatch visible to the optimizer. In
// particular, a static account name needs neither allocation nor an indirect call.
#[derive(Clone)]
#[repr(u64)]
enum Origin {
    Account(&'static str),
    OwnedAccount(Box<str>),
    AccountKey(Pubkey),
    Source(Source),
}
impl Origin {
    fn account(name: &'static str) -> Self { Self::Account(name) }
    fn owned_account(name: &str) -> Self { Self::OwnedAccount(name.into()) }
    fn account_key(key: Pubkey) -> Self { Self::AccountKey(key) }
    fn source(source: Source) -> Self { Self::Source(source) }
}
impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Account(name) => write!(f, "caused by account: {}", name),
            Self::OwnedAccount(name) => write!(f, "caused by account: {}", name),
            Self::AccountKey(key) => write!(f, "caused by account: {}", KeyDisplay(key)),
            Self::Source(source) => write!(f, "thrown in {}:{}", source.filename, source.line),
        }
    }
}
impl fmt::Debug for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Account(name) => f.debug_tuple("Account").field(name).finish(),
            Self::OwnedAccount(name) => f.debug_tuple("Account").field(name).finish(),
            Self::AccountKey(key) => f.debug_tuple("AccountKey").field(&KeyDisplay(key)).finish(),
            Self::Source(source) => f.debug_tuple("Source").field(source).finish(),
        }
    }
}
// Error origins need only the canonical fixed 32-byte address encoding.
// Keeping this formatter local avoids pulling the SDK's general formatting
// implementation into programs which never otherwise display a public key.
struct KeyDisplay<'a>(&'a Pubkey);
#[inline(never)]
fn format_key(key: &Pubkey) -> ([u8; 44], usize) {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut number = key.to_bytes();
    let leading_zeroes = number.iter().take_while(|byte| **byte == 0).count();
    let mut start = leading_zeroes;
    let mut output = [0u8; 44];
    let mut end = output.len();
    while start < number.len() {
        let mut remainder = 0u16;
        for byte in &mut number[start..] {
            let value = (remainder << 8) | u16::from(*byte);
            *byte = (value / 58) as u8;
            remainder = value % 58;
        }
        end -= 1;
        output[end] = ALPHABET[remainder as usize];
        while start < number.len() && number[start] == 0 { start += 1; }
    }
    for _ in 0..leading_zeroes { end -= 1; output[end] = b'1'; }
    (output, end)
}
impl fmt::Display for KeyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (output, start) = format_key(self.0);
        // Every byte comes from the ASCII base58 alphabet.
        f.write_str(unsafe { std::str::from_utf8_unchecked(&output[start..]) })
    }
}
impl fmt::Debug for KeyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self, f) }
}
struct OriginMessage<'a>(Option<&'a Origin>);
impl fmt::Display for OriginMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 { Some(origin) => fmt::Display::fmt(origin, f), None => f.write_str("occurred") }
    }
}
#[derive(Clone)]
#[repr(u64)]
enum Compared {
    Pubkeys((Pubkey, Pubkey)),
    Values((CompactRendered, CompactRendered)),
    Formatted(Box<(Rendered, Rendered)>),
}
#[derive(Clone)]
struct CompactRendered {
    bytes: [u8; 24],
    len: u8,
    format: RenderFormat,
}
#[derive(Clone, Copy)]
enum RenderFormat { Text, Decimal, NegativeDecimal }
impl CompactRendered {
    fn from_text(text: &str) -> Option<Self> {
        let mut value=Self {bytes:[0;24],len:0,format:RenderFormat::Text};
        if text.len()<=value.bytes.len() {
            value.bytes[..text.len()].copy_from_slice(text.as_bytes());
            value.len=text.len() as u8;
            return Some(value);
        }
        let negative=text.starts_with('-');
        let digits=&text.as_bytes()[usize::from(negative)..];
        if digits.is_empty() || digits.len()>48 || !digits.iter().all(u8::is_ascii_digit) {return None;}
        value.format=if negative {RenderFormat::NegativeDecimal} else {RenderFormat::Decimal};
        value.len=digits.len() as u8;
        for (index,digit) in digits.iter().enumerate() {
            value.bytes[index/2]|=(digit-b'0')<<((index%2)*4);
        }
        Some(value)
    }
}
impl fmt::Display for CompactRendered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if matches!(self.format,RenderFormat::Text) {
            // Source text is UTF-8 and is copied whole, never truncated.
            return f.write_str(unsafe {std::str::from_utf8_unchecked(&self.bytes[..self.len as usize])});
        }
        let mut text=[0u8;49];
        let start=usize::from(matches!(self.format,RenderFormat::NegativeDecimal));
        if start!=0 {text[0]=b'-';}
        for index in 0..self.len as usize {
            text[start+index]=b'0'+((self.bytes[index/2]>>((index%2)*4))&15);
        }
        f.write_str(unsafe {std::str::from_utf8_unchecked(&text[..start+self.len as usize])})
    }
}
impl fmt::Debug for Compared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pubkeys(keys)=>f.debug_tuple("Pubkeys").field(keys).finish(),
            Self::Values((left,right))=>f.debug_tuple("Values").field(&(Rendered::new(left),Rendered::new(right))).finish(),
            Self::Formatted(values)=>f.debug_tuple("Values").field(&**values).finish(),
        }
    }
}
#[derive(Debug, Clone)]
enum Rendered {
    Inline(StackString<64>),
    Owned(String),
}
impl Rendered {
    fn new(value: impl fmt::Display) -> Self {
        let mut text = StackString::<64>::new();
        if write!(text, "{}", value).is_ok() {
            Self::Inline(text)
        } else {
            Self::Owned(format!("{}", value))
        }
    }
    fn as_str(&self) -> &str {
        match self {
            Self::Inline(s) => s.as_str(),
            Self::Owned(s) => s,
        }
    }
}
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ErrorDescriptor {
    pub code: u32,
    pub name: &'static str,
    pub message: &'static str,
}
#[derive(Clone)]
#[repr(u64)]
enum ErrorKind {
    Anchor(&'static ErrorDescriptor),
    Program(ProgramError),
    Owned(Box<ErrorDescriptor>),
}
#[derive(Clone)]
#[repr(C)]
struct ErrorInner {
    kind: ErrorKind,
    origin: Option<Origin>,
    compared: Option<Compared>,
}
#[repr(C)]
pub struct Error {
    storage: std::mem::MaybeUninit<ErrorInner>,
}
impl Clone for Error {
    fn clone(&self) -> Self { Self::from_inner(self.inner().clone()) }
}
impl Drop for Error {
    fn drop(&mut self) { unsafe { self.storage.assume_init_drop(); } }
}
impl Error {
    #[inline(always)]
    fn from_inner(inner: ErrorInner) -> Self { Self { storage: std::mem::MaybeUninit::new(inner) } }
    #[inline(always)]
    fn inner(&self) -> &ErrorInner { unsafe { self.storage.assume_init_ref() } }
    #[inline(always)]
    fn inner_mut(&mut self) -> &mut ErrorInner { unsafe { self.storage.assume_init_mut() } }
    #[inline(always)]
    fn into_inner(self) -> ErrorInner {
        let value=std::mem::ManuallyDrop::new(self);
        unsafe { value.storage.assume_init_read() }
    }
}
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (&self.inner().kind, &other.inner().kind) {
            (ErrorKind::Program(left), ErrorKind::Program(right)) => left == right,
            (ErrorKind::Program(_), _) | (_, ErrorKind::Program(_)) => false,
            _ => self.descriptor().code == other.descriptor().code,
        }
    }
}
impl Eq for Error {}
impl Error {
    pub fn new(code: u32, name: &'static str, message: &'static str) -> Self {
        Self::from_inner(ErrorInner { kind: ErrorKind::Owned(Box::new(ErrorDescriptor {code,name,message})), origin: None, compared: None })
    }
    #[doc(hidden)]
    pub fn from_static(descriptor: &'static ErrorDescriptor) -> Self {
        Self::from_inner(ErrorInner { kind: ErrorKind::Anchor(descriptor), origin: None, compared: None })
    }
    fn descriptor(&self) -> &ErrorDescriptor {
        static EMPTY: ErrorDescriptor = ErrorDescriptor {code:0,name:"",message:""};
        match &self.inner().kind {
            ErrorKind::Anchor(descriptor)=>descriptor,
            ErrorKind::Owned(descriptor)=>descriptor,
            ErrorKind::Program(_)=>&EMPTY,
        }
    }
    fn program_error(&self) -> Option<&ProgramError> {
        match &self.inner().kind {ErrorKind::Program(error)=>Some(error),_=>None}
    }
    pub fn with_account_name(mut self, name: &str) -> Self {
        self.inner_mut().origin = Some(Origin::owned_account(name));
        self
    }
    #[doc(hidden)]
    pub fn __tare_with_account_name_static(mut self, name: &'static str) -> Self {
        self.inner_mut().origin = Some(Origin::account(name));
        self
    }
    pub fn with_account_key(mut self, key: Pubkey) -> Self {
        self.inner_mut().origin = Some(Origin::account_key(key));
        self
    }
    pub fn with_source(mut self, source: Source) -> Self {
        self.inner_mut().origin = Some(Origin::source(source));
        self
    }
    pub fn with_pubkeys(mut self, keys: (Pubkey, Pubkey)) -> Self {
        self.inner_mut().compared = Some(Compared::Pubkeys(keys));
        self
    }
    pub fn with_values(mut self, values: (impl fmt::Display, impl fmt::Display)) -> Self {
        let left=Rendered::new(values.0);
        let right=Rendered::new(values.1);
        self.inner_mut().compared=Some(match (CompactRendered::from_text(left.as_str()),CompactRendered::from_text(right.as_str())) {
            (Some(left),Some(right))=>Compared::Values((left,right)),
            _=>Compared::Formatted(Box::new((left,right))),
        });
        self
    }
    fn write_line(&self, buffer: &mut impl Write) -> fmt::Result {
        let origin = OriginMessage(self.inner().origin.as_ref());
        if let Some(program) = self.program_error() {
            write!(buffer, "ProgramError {}. Error Code: {:?}. Error Number: {}. Error Message: {}.", origin, program, u64::from(program.clone()), program)
        } else {
            write!(buffer, "AnchorError {}. Error Code: {}. Error Number: {}. Error Message: {}.", origin, self.descriptor().name, self.descriptor().code, self.descriptor().message)
        }
    }
    #[inline(never)]
    pub fn log(&self) {
        let mut buffer = StackString::<1024>::new();
        if self.write_line(&mut buffer).is_err() {
            let mut full = String::new();
            self.write_line(&mut full).expect("String writes succeed");
            solana_program::log::sol_log(&full);
        } else {
            solana_program::log::sol_log(buffer.as_str());
        }
        match &self.inner().compared {
            Some(Compared::Pubkeys((left, right))) => {
                solana_program::log::sol_log("Left:");
                left.log();
                solana_program::log::sol_log("Right:");
                right.log();
            }
            Some(Compared::Values((left, right))) => {
                crate::log_fmt(format_args!("Left: {}", left));
                crate::log_fmt(format_args!("Right: {}", right));
            }
            Some(Compared::Formatted(values)) => {
                crate::log_fmt(format_args!("Left: {}", values.0.as_str()));
                crate::log_fmt(format_args!("Right: {}", values.1.as_str()));
            }
            None => {}
        }
    }
}
impl From<ProgramError> for Error {
    fn from(program: ProgramError) -> Self {
        Self::from_inner(ErrorInner {kind:ErrorKind::Program(program),origin:None,compared:None})
    }
}
impl From<Error> for ProgramError {
    fn from(error: Error) -> Self {
        match error.into_inner().kind {
            ErrorKind::Program(program)=>program,
            ErrorKind::Anchor(descriptor)=>ProgramError::Custom(descriptor.code),
            ErrorKind::Owned(descriptor)=>ProgramError::Custom(descriptor.code),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.descriptor().name, self.descriptor().message)
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f:&mut fmt::Formatter<'_>)->fmt::Result {
        let descriptor=self.descriptor();
        f.debug_struct("Error").field("code",&descriptor.code).field("name",&descriptor.name).field("message",&descriptor.message)
            .field("origin",&self.inner().origin).field("compared",&self.inner().compared).field("program",&self.program_error()).finish()
    }
}
impl std::error::Error for Error {}
#[derive(Debug, Clone)]
pub struct StackString<const N: usize> {
    bytes: [u8; N],
    len: usize,
}
impl<const N: usize> StackString<N> {
    pub fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }
    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.bytes[..self.len]) }
    }
}
impl<const N: usize> Write for StackString<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.len + text.len() > N {
            return Err(fmt::Error);
        }
        self.bytes[self.len..self.len + text.len()].copy_from_slice(text.as_bytes());
        self.len += text.len();
        Ok(())
    }
}

// Error identifiers and messages are the Anchor 0.31.1 public contract (Apache-2.0).
#[error_code(offset = 0)]
pub enum ErrorCode {
    // Instructions
    /// 100 - Instruction discriminator not provided
    #[msg("Instruction discriminator not provided")]
    InstructionMissing = 100,
    /// 101 - Fallback functions are not supported
    #[msg("Fallback functions are not supported")]
    InstructionFallbackNotFound,
    /// 102 - The program could not deserialize the given instruction
    #[msg("The program could not deserialize the given instruction")]
    InstructionDidNotDeserialize,
    /// 103 - The program could not serialize the given instruction
    #[msg("The program could not serialize the given instruction")]
    InstructionDidNotSerialize,

    // IDL instructions
    /// 1000 - The program was compiled without idl instructions
    #[msg("The program was compiled without idl instructions")]
    IdlInstructionStub = 1000,
    /// 1001 - Invalid program given to the IDL instruction
    #[msg("Invalid program given to the IDL instruction")]
    IdlInstructionInvalidProgram,
    /// 1002 - IDL Account must be empty in order to resize
    #[msg("IDL account must be empty in order to resize, try closing first")]
    IdlAccountNotEmpty,

    // Event instructions
    /// 1500 - The program was compiled without `event-cpi` feature
    #[msg("The program was compiled without `event-cpi` feature")]
    EventInstructionStub = 1500,

    // Constraints
    /// 2000 - A mut constraint was violated
    #[msg("A mut constraint was violated")]
    ConstraintMut = 2000,
    /// 2001 - A has one constraint was violated
    #[msg("A has one constraint was violated")]
    ConstraintHasOne,
    /// 2002 - A signer constraint was violated
    #[msg("A signer constraint was violated")]
    ConstraintSigner,
    /// 2003 - A raw constraint was violated
    #[msg("A raw constraint was violated")]
    ConstraintRaw,
    /// 2004 - An owner constraint was violated
    #[msg("An owner constraint was violated")]
    ConstraintOwner,
    /// 2005 - A rent exemption constraint was violated
    #[msg("A rent exemption constraint was violated")]
    ConstraintRentExempt,
    /// 2006 - A seeds constraint was violated
    #[msg("A seeds constraint was violated")]
    ConstraintSeeds,
    /// 2007 - An executable constraint was violated
    #[msg("An executable constraint was violated")]
    ConstraintExecutable,
    /// 2008 - Deprecated Error, feel free to replace with something else
    #[msg("Deprecated Error, feel free to replace with something else")]
    ConstraintState,
    /// 2009 - An associated constraint was violated
    #[msg("An associated constraint was violated")]
    ConstraintAssociated,
    /// 2010 - An associated init constraint was violated
    #[msg("An associated init constraint was violated")]
    ConstraintAssociatedInit,
    /// 2011 - A close constraint was violated
    #[msg("A close constraint was violated")]
    ConstraintClose,
    /// 2012 - An address constraint was violated
    #[msg("An address constraint was violated")]
    ConstraintAddress,
    /// 2013 - Expected zero account discriminant
    #[msg("Expected zero account discriminant")]
    ConstraintZero,
    /// 2014 - A token mint constraint was violated
    #[msg("A token mint constraint was violated")]
    ConstraintTokenMint,
    /// 2015 - A token owner constraint was violated
    #[msg("A token owner constraint was violated")]
    ConstraintTokenOwner,
    /// The mint mint is intentional -> a mint authority for the mint.
    ///
    /// 2016 - A mint mint authority constraint was violated
    #[msg("A mint mint authority constraint was violated")]
    ConstraintMintMintAuthority,
    /// 2017 - A mint freeze authority constraint was violated
    #[msg("A mint freeze authority constraint was violated")]
    ConstraintMintFreezeAuthority,
    /// 2018 - A mint decimals constraint was violated
    #[msg("A mint decimals constraint was violated")]
    ConstraintMintDecimals,
    /// 2019 - A space constraint was violated
    #[msg("A space constraint was violated")]
    ConstraintSpace,
    /// 2020 - A required account for the constraint is None
    #[msg("A required account for the constraint is None")]
    ConstraintAccountIsNone,
    /// The token token is intentional -> a token program for the token account.
    ///
    /// 2021 - A token account token program constraint was violated
    #[msg("A token account token program constraint was violated")]
    ConstraintTokenTokenProgram,
    /// 2022 - A mint token program constraint was violated
    #[msg("A mint token program constraint was violated")]
    ConstraintMintTokenProgram,
    /// 2023 - A mint token program constraint was violated
    #[msg("An associated token account token program constraint was violated")]
    ConstraintAssociatedTokenTokenProgram,
    /// Extension constraints
    ///
    /// 2024 - A group pointer extension constraint was violated
    #[msg("A group pointer extension constraint was violated")]
    ConstraintMintGroupPointerExtension,
    /// 2025 - A group pointer extension authority constraint was violated
    #[msg("A group pointer extension authority constraint was violated")]
    ConstraintMintGroupPointerExtensionAuthority,
    /// 2026 - A group pointer extension group address constraint was violated
    #[msg("A group pointer extension group address constraint was violated")]
    ConstraintMintGroupPointerExtensionGroupAddress,
    /// 2027 - A group member pointer extension constraint was violated
    #[msg("A group member pointer extension constraint was violated")]
    ConstraintMintGroupMemberPointerExtension,
    /// 2028 - A group member pointer extension authority constraint was violated
    #[msg("A group member pointer extension authority constraint was violated")]
    ConstraintMintGroupMemberPointerExtensionAuthority,
    /// 2029 - A group member pointer extension member address constraint was violated
    #[msg("A group member pointer extension group address constraint was violated")]
    ConstraintMintGroupMemberPointerExtensionMemberAddress,
    /// 2030 - A metadata pointer extension constraint was violated
    #[msg("A metadata pointer extension constraint was violated")]
    ConstraintMintMetadataPointerExtension,
    /// 2031 - A metadata pointer extension authority constraint was violated
    #[msg("A metadata pointer extension authority constraint was violated")]
    ConstraintMintMetadataPointerExtensionAuthority,
    /// 2032 - A metadata pointer extension metadata address constraint was violated
    #[msg("A metadata pointer extension metadata address constraint was violated")]
    ConstraintMintMetadataPointerExtensionMetadataAddress,
    /// 2033 - A close authority extension constraint was violated
    #[msg("A close authority constraint was violated")]
    ConstraintMintCloseAuthorityExtension,
    /// 2034 - A close authority extension authority constraint was violated
    #[msg("A close authority extension authority constraint was violated")]
    ConstraintMintCloseAuthorityExtensionAuthority,
    /// 2035 - A permanent delegate extension constraint was violated
    #[msg("A permanent delegate extension constraint was violated")]
    ConstraintMintPermanentDelegateExtension,
    /// 2036 - A permanent delegate extension authority constraint was violated
    #[msg("A permanent delegate extension delegate constraint was violated")]
    ConstraintMintPermanentDelegateExtensionDelegate,
    /// 2037 - A transfer hook extension constraint was violated
    #[msg("A transfer hook extension constraint was violated")]
    ConstraintMintTransferHookExtension,
    /// 2038 - A transfer hook extension authority constraint was violated
    #[msg("A transfer hook extension authority constraint was violated")]
    ConstraintMintTransferHookExtensionAuthority,
    /// 2039 - A transfer hook extension transfer hook program id constraint was violated
    #[msg("A transfer hook extension transfer hook program id constraint was violated")]
    ConstraintMintTransferHookExtensionProgramId,

    // Require
    /// 2500 - A require expression was violated
    #[msg("A require expression was violated")]
    RequireViolated = 2500,
    /// 2501 - A require_eq expression was violated
    #[msg("A require_eq expression was violated")]
    RequireEqViolated,
    /// 2502 - A require_keys_eq expression was violated
    #[msg("A require_keys_eq expression was violated")]
    RequireKeysEqViolated,
    /// 2503 - A require_neq expression was violated
    #[msg("A require_neq expression was violated")]
    RequireNeqViolated,
    /// 2504 - A require_keys_neq expression was violated
    #[msg("A require_keys_neq expression was violated")]
    RequireKeysNeqViolated,
    /// 2505 - A require_gt expression was violated
    #[msg("A require_gt expression was violated")]
    RequireGtViolated,
    /// 2506 - A require_gte expression was violated
    #[msg("A require_gte expression was violated")]
    RequireGteViolated,

    // Accounts.
    /// 3000 - The account discriminator was already set on this account
    #[msg("The account discriminator was already set on this account")]
    AccountDiscriminatorAlreadySet = 3000,
    /// 3001 - No discriminator was found on the account
    #[msg("No discriminator was found on the account")]
    AccountDiscriminatorNotFound,
    /// 3002 - Account discriminator did not match what was expected
    #[msg("Account discriminator did not match what was expected")]
    AccountDiscriminatorMismatch,
    /// 3003 - Failed to deserialize the account
    #[msg("Failed to deserialize the account")]
    AccountDidNotDeserialize,
    /// 3004 - Failed to serialize the account
    #[msg("Failed to serialize the account")]
    AccountDidNotSerialize,
    /// 3005 - Not enough account keys given to the instruction
    #[msg("Not enough account keys given to the instruction")]
    AccountNotEnoughKeys,
    /// 3006 - The given account is not mutable
    #[msg("The given account is not mutable")]
    AccountNotMutable,
    /// 3007 - The given account is owned by a different program than expected
    #[msg("The given account is owned by a different program than expected")]
    AccountOwnedByWrongProgram,
    /// 3008 - Program ID was not as expected
    #[msg("Program ID was not as expected")]
    InvalidProgramId,
    /// 3009 - Program account is not executable
    #[msg("Program account is not executable")]
    InvalidProgramExecutable,
    /// 3010 - The given account did not sign
    #[msg("The given account did not sign")]
    AccountNotSigner,
    /// 3011 - The given account is not owned by the system program
    #[msg("The given account is not owned by the system program")]
    AccountNotSystemOwned,
    /// 3012 - The program expected this account to be already initialized
    #[msg("The program expected this account to be already initialized")]
    AccountNotInitialized,
    /// 3013 - The given account is not a program data account
    #[msg("The given account is not a program data account")]
    AccountNotProgramData,
    /// 3014 - The given account is not the associated token account
    #[msg("The given account is not the associated token account")]
    AccountNotAssociatedTokenAccount,
    /// 3015 - The given public key does not match the required sysvar
    #[msg("The given public key does not match the required sysvar")]
    AccountSysvarMismatch,
    /// 3016 - The account reallocation exceeds the MAX_PERMITTED_DATA_INCREASE limit
    #[msg("The account reallocation exceeds the MAX_PERMITTED_DATA_INCREASE limit")]
    AccountReallocExceedsLimit,
    /// 3017 - The account was duplicated for more than one reallocation
    #[msg("The account was duplicated for more than one reallocation")]
    AccountDuplicateReallocs,

    // Miscellaneous
    /// 4100 - The declared program id does not match actual program id
    #[msg("The declared program id does not match the actual program id")]
    DeclaredProgramIdMismatch = 4100,
    /// 4101 - You cannot/should not initialize the payer account as a program account
    #[msg("You cannot/should not initialize the payer account as a program account")]
    TryingToInitPayerAsProgramAccount = 4101,
    /// 4102 - Invalid numeric conversion error
    #[msg("Error during numeric conversion")]
    InvalidNumericConversion = 4102,

    // Deprecated
    /// 5000 - The API being used is deprecated and should no longer be used
    #[msg("The API being used is deprecated and should no longer be used")]
    Deprecated = 5000,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        solana_program::program_error::ProgramError::BorshIoError(value.to_string()).into()
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(_:std::num::TryFromIntError)->Self {
        Self::from_static(&ErrorDescriptor {code:4102,name:"InvalidNumericConversion",message:"out of range integral type conversion attempted"})
    }
}
