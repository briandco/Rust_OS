// Core types for the RTOS

/// Priority type - higher number = higher priority
/// Range: 0 (idle) to MAX_PRIORITIES-1 (highest)
pub type Priority = usize;

/// Tick counter type - wraps around for overflow handling
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TickType(pub u64);

impl TickType{
    pub const fn new(value: u64) -> Self{
        TickType(value)
    }

    pub const fn zero() -> Self{
        TickType(0)
    }

    pub const fn max() -> Self{
        TickType(u64::MAX)  
    }

    pub fn wrapping_add(self, other: TickType) -> TickType{
        TickType(self.0.wrapping_add(other.0))
    }
    
    pub fn elapsed_since(self, earlier: TickType) -> TickType{
        TickType(self.0.wrapping_sub(earlier.0))
    }

    /// Convert from milliseconds (assuming 1ms tick)
    pub fn from_ms(ms: u64) -> Self{
        TickType(ms)
    }

    /// Convert to milliseconds (assuming 1ms tick)
    pub fn to_ms(self) -> u64{
        self.0
    }
}

/// Task states
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskState{
    Ready,
    Running,
    Blocked,
    Suspended,
    Deleted,
}

pub type StackSize = usize;

/// Error types for RTOS operations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RtosError{
    OutOfMemory,
    InvalidPriority,
    TaskNotFound,
    InvalidParameter,
    Timeout,
    ResourceBusy,
}

pub type Result<T> = core::result::Result<T, RtosError>;

//Configuration constants
pub mod config{
    use super::*;

    /// Maximum number of priority levels
    pub const MAX_PRIORITIES: usize = 32;
    
    /// Idle task priority (always 0)
    pub const IDLE_PRIORITY: Priority = 0;
    
    /// Default task stack size (in words)
    pub const DEFAULT_STACK_SIZE: StackSize = 1024;
    
    /// Minimum task stack size (in words)
    pub const MIN_STACK_SIZE: StackSize = 256;
    
    /// System tick frequency in Hz
    pub const TICK_RATE_HZ: u64 = 1000; // 1ms tick
    
    /// Enable/disable preemption
    pub const USE_PREEMPTION: bool = true;
    
    /// Enable/disable time slicing
    pub const USE_TIME_SLICING: bool = true;
    
    /// Stack fill pattern for debugging
    pub const STACK_FILL_BYTE: u8 = 0xa5;
}


