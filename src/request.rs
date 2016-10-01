//! Request objects for non-blocking operations
//!
//! Non-blocking operations such as `immediate_send()` return request objects that borrow any
//! buffers involved in the operation so as to ensure proper access restrictions. In order to
//! release the borrowed buffers from the request objects, a completion operation such as `wait()`
//! or `test()` must be used on the request object. To enforce this rule, the request objects
//! implement a `Drop` bomb which will `panic!()` when a request object is dropped.
//!
//! To handle request completion in a RAII style, requests can be wrapped in either `WaitGuard` or
//! `CancelGuard` which will follow the respective policy for completing the operation upon being
//! dropped instead of `panic!()`ing.
//!
//! # Unfinished features
//!
//! - **3.7**: Nonblocking mode:
//!   - Completion, `MPI_Waitany()`, `MPI_Waitall()`, `MPI_Waitsome()`,
//!   `MPI_Testany()`, `MPI_Testall()`, `MPI_Testsome()`, `MPI_Request_get_status()`
//! - **3.8**:
//!   - Cancellation, `MPI_Cancel()`, `MPI_Test_cancelled()`

use std::mem;
use std::marker::PhantomData;
use std::os::raw::c_int;

use ffi;
use ffi::{MPI_Request, MPI_Status};

use point_to_point::Status;
use raw::traits::*;

/// Request object traits
pub mod traits {
    pub use super::Request;
}

/// A request for a non-blocking operation
pub trait Request: AsRaw<Raw = MPI_Request> + AsRawMut {
    /// Returns true for a null request handle.
    fn is_null(&self) -> bool {
        self.as_raw() == unsafe { ffi::RSMPI_REQUEST_NULL }
    }

    /// Wait for an operation to finish.
    ///
    /// Will block execution of the calling thread until the associated operation has finished.
    ///
    /// # Examples
    ///
    /// See `examples/immediate.rs`
    ///
    /// # Standard section(s)
    ///
    /// 3.7.3
    fn wait(mut self) -> Status
        where Self: Sized
    {
        let mut status: MPI_Status = unsafe { mem::uninitialized() };
        unsafe {
            ffi::MPI_Wait(self.as_raw_mut(), &mut status);
        }
        assert!(self.is_null());
        mem::forget(self);
        Status::from_raw(status)
    }

    /// Test whether an operation has finished.
    ///
    /// If the operation has finished returns the `Status` otherwise returns the unfinished
    /// `Request`.
    /// # Examples
    ///
    /// See `examples/immediate.rs`
    ///
    /// # Standard section(s)
    ///
    /// 3.7.3
    fn test(mut self) -> Result<Status, Self>
        where Self: Sized
    {
        let mut status: MPI_Status = unsafe { mem::uninitialized() };
        let mut flag: c_int = 0;
        unsafe {
            ffi::MPI_Test(self.as_raw_mut(), &mut flag, &mut status);
        }
        assert!(flag == 0 || self.is_null());
        if flag != 0 {
            mem::forget(self);
            Ok(Status::from_raw(status))
        } else {
            Err(self)
        }
    }

    /// Cancel an operation.
    ///
    /// # Examples
    ///
    /// See `examples/immediate.rs`
    ///
    /// # Standard section(s)
    ///
    /// 3.8.4
    fn cancel(mut self)
        where Self: Sized
    {
        unsafe {
            ffi::MPI_Cancel(self.as_raw_mut());
            ffi::MPI_Request_free(self.as_raw_mut());
        }
        assert!(self.is_null());
        mem::forget(self);
    }
}

/// A request object for an non-blocking operation that holds no references
///
/// # Examples
///
/// See `examples/immediate_barrier.rs`
///
/// # Standard section(s)
///
/// 3.7.1
#[must_use]
pub struct PlainRequest(MPI_Request);

impl PlainRequest {
    /// Construct a request object from the raw MPI type
    pub fn from_raw(request: MPI_Request) -> PlainRequest {
        PlainRequest(request)
    }
}

unsafe impl AsRaw for PlainRequest {
    type Raw = MPI_Request;
    fn as_raw(&self) -> Self::Raw {
        self.0
    }
}

unsafe impl AsRawMut for PlainRequest {
    fn as_raw_mut(&mut self) -> *mut <Self as AsRaw>::Raw {
        &mut (self.0)
    }
}

impl Request for PlainRequest {}

impl Drop for PlainRequest {
    fn drop(&mut self) {
        assert!(self.is_null(),
                "request dropped without ascertaining completion.");
    }
}

/// A request object for a non-blocking operation that holds a reference to an immutable buffer
///
/// # Examples
///
/// See `examples/immediate.rs`
///
/// # Standard section(s)
///
/// 3.7.1
#[must_use]
pub struct ReadRequest<'b, Buf: 'b + ?Sized>(MPI_Request, PhantomData<&'b Buf>);

impl<'b, Buf: 'b + ?Sized> ReadRequest<'b, Buf> {
    /// Construct a request object from the raw MPI type
    pub fn from_raw(request: MPI_Request, _: &'b Buf) -> ReadRequest<'b, Buf> {
        ReadRequest(request, PhantomData)
    }
}

unsafe impl<'b, Buf: 'b + ?Sized> AsRaw for ReadRequest<'b, Buf> {
    type Raw = MPI_Request;
    fn as_raw(&self) -> Self::Raw {
        self.0
    }
}

unsafe impl<'b, Buf: 'b + ?Sized> AsRawMut for ReadRequest<'b, Buf> {
    fn as_raw_mut(&mut self) -> *mut <Self as AsRaw>::Raw {
        &mut (self.0)
    }
}

impl<'b, Buf: 'b + ?Sized> Request for ReadRequest<'b, Buf> {}

impl<'b, Buf: 'b + ?Sized> Drop for ReadRequest<'b, Buf> {
    fn drop(&mut self) {
        assert!(self.is_null(),
                "read request dropped without ascertaining completion.");
    }
}

/// A request object for a non-blocking operation that holds a reference to a mutable buffer
///
/// # Examples
///
/// See `examples/immediate.rs`
///
/// # Standard section(s)
///
/// 3.7.1
#[must_use]
pub struct WriteRequest<'b, Buf: 'b + ?Sized>(MPI_Request, PhantomData<&'b mut Buf>);

impl<'b, Buf: 'b + ?Sized> WriteRequest<'b, Buf> {
    /// Construct a request object from the raw MPI type
    pub fn from_raw(request: MPI_Request, _: &'b Buf) -> WriteRequest<'b, Buf> {
        WriteRequest(request, PhantomData)
    }
}

unsafe impl<'b, Buf: 'b + ?Sized> AsRaw for WriteRequest<'b, Buf> {
    type Raw = MPI_Request;
    fn as_raw(&self) -> Self::Raw {
        self.0
    }
}

unsafe impl<'b, Buf: 'b + ?Sized> AsRawMut for WriteRequest<'b, Buf> {
    fn as_raw_mut(&mut self) -> *mut <Self as AsRaw>::Raw {
        &mut (self.0)
    }
}

impl<'b, Buf: 'b + ?Sized> Request for WriteRequest<'b, Buf> {}

impl<'b, Buf: 'b + ?Sized> Drop for WriteRequest<'b, Buf> {
    fn drop(&mut self) {
        assert!(self.is_null(),
                "write request dropped without ascertaining completion.");
    }
}

/// A request object for a non-blocking operation that holds a reference to a mutable and an
/// immutable buffer
///
/// # Examples
///
/// See `examples/immediate_gather.rs`
///
/// # Standard section(s)
///
/// 3.7.1
#[must_use]
pub struct ReadWriteRequest<'s, 'r, S: 's + ?Sized, R: 'r + ?Sized>(MPI_Request,
                                                                    PhantomData<&'s S>,
                                                                    PhantomData<&'r mut R>);

impl<'s, 'r, S: 's + ?Sized, R: 'r + ?Sized> ReadWriteRequest<'s, 'r, S, R> {
    /// Construct a request object from the raw MPI type
    pub fn from_raw(request: MPI_Request, _: &'s S, _: &'r R) -> ReadWriteRequest<'s, 'r, S, R> {
        ReadWriteRequest(request, PhantomData, PhantomData)
    }
}

unsafe impl<'s, 'r, S: 's + ?Sized, R: 'r + ?Sized> AsRaw for ReadWriteRequest<'s, 'r, S, R> {
    type Raw = MPI_Request;
    fn as_raw(&self) -> Self::Raw {
        self.0
    }
}

unsafe impl<'s, 'r, S: 's + ?Sized, R: 'r + ?Sized> AsRawMut for ReadWriteRequest<'s, 'r, S, R> {
    fn as_raw_mut(&mut self) -> *mut <Self as AsRaw>::Raw {
        &mut (self.0)
    }
}

impl<'s, 'r, S: 's + ?Sized, R: 'r + ?Sized> Request for ReadWriteRequest<'s, 'r, S, R> {}

impl<'s, 'r, S: 's + ?Sized, R: 'r + ?Sized> Drop for ReadWriteRequest<'s, 'r, S, R> {
    fn drop(&mut self) {
        assert!(self.is_null(),
                "read-write request dropped without ascertaining completion.");
    }
}

/// Guard object that waits for the completion of an operation when it is dropped
///
/// # Examples
///
/// See `examples/immediate.rs`
pub struct WaitGuard<Req>(Option<Req>) where Req: Request;

impl<Req> Drop for WaitGuard<Req> where Req: Request
{
    fn drop(&mut self) {
        self.0.take().map(|mut req| {
            unsafe {
                ffi::MPI_Wait(req.as_raw_mut(), ffi::RSMPI_STATUS_IGNORE);
            }
            assert!(req.is_null());
            mem::forget(req);
        });
    }
}

impl<Req> From<Req> for WaitGuard<Req> where Req: Request
{
    fn from(req: Req) -> WaitGuard<Req> {
        WaitGuard(Some(req))
    }
}

/// Guard object that cancels an operation when it is dropped
///
/// # Examples
///
/// See `examples/immediate.rs`
pub struct CancelGuard<Req>(Option<Req>) where Req: Request;

impl<Req> Drop for CancelGuard<Req> where Req: Request
{
    fn drop(&mut self) {
        self.0.take().map(|mut req| {
            unsafe {
                ffi::MPI_Cancel(req.as_raw_mut());
                ffi::MPI_Request_free(req.as_raw_mut());
            }
            assert!(req.is_null());
            mem::forget(req);
        });
    }
}

impl<Req> From<Req> for CancelGuard<Req> where Req: Request
{
    fn from(req: Req) -> CancelGuard<Req> {
        CancelGuard(Some(req))
    }
}
