use crate::{Observable, Observer, Subscription};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) type CallbackPtr<'a, T> = *const (dyn for<'r> FnMut(&'r T) + 'a);

type CallbackVec<'a, T> = Rc<RefCell<Vec<Box<FnMut(&T) + 'a>>>>;

#[derive(Default)]
pub struct Subject<'a, T> {
  callbacks: CallbackVec<'a, T>,
}

impl<'a, T> Clone for Subject<'a, T> {
  fn clone(&self) -> Self {
    Subject {
      callbacks: self.callbacks.clone(),
    }
  }
}

impl<'a, T: 'a> Observable<'a> for Subject<'a, T> {
  type Item = &'a T;
  type Unsubscribe = SubjectSubscription<'a, T>;

  fn subscribe<O>(self, observer: O) -> Self::Unsubscribe
  where
    O: FnMut(Self::Item) + 'a,
  {
    let observer: Box<FnMut(Self::Item)> = Box::new(observer);
    // of course, we know Self::Item and &'a T is the same type, but
    // rust can't infer it, so, write an unsafe code to let rust know.
    let observer: Box<(dyn for<'r> std::ops::FnMut(&'r T) + 'a)> =
      unsafe { std::mem::transmute(observer) };
    let ptr = observer.as_ref() as CallbackPtr<T>;
    self.callbacks.borrow_mut().push(observer);

    SubjectSubscription {
      source: self,
      callback: ptr,
    }
  }
}

impl<'a, T: 'a> Subject<'a, T> {
  pub fn new() -> Subject<'a, T> {
    Subject {
      callbacks: Rc::new(RefCell::new(vec![])),
    }
  }

  /// Create a new subject from a stream, enabling multiple observers
  /// ("fork" the stream)
  pub fn from_stream<S>(stream: S) -> Self
  where
    S: Observable<'a, Item = T>,
  {
    let broadcast = Self::new();
    let clone = broadcast.clone();

    stream.subscribe(move |x| {
      clone.next(x);
    });
    broadcast
  }

  pub fn remove_callback(&mut self, ptr: CallbackPtr<T>) {
    self
      .callbacks
      .borrow_mut()
      .retain(|x| x.as_ref() as *const _ != ptr);
  }
}

impl<'a, T> Observer for Subject<'a, T> {
  type Item = T;

  fn next(&self, v: Self::Item) -> &Self {
    for observer in self.callbacks.borrow_mut().iter_mut() {
      observer(&v);
    }
    self
  }
}

pub struct SubjectSubscription<'a, T> {
  source: Subject<'a, T>,
  callback: CallbackPtr<'a, T>,
}

impl<'a, T: 'a> Subscription for SubjectSubscription<'a, T> {
  fn unsubscribe(mut self) { self.source.remove_callback(self.callback); }
}

#[test]
fn base_data_flow() {
  let mut i = 0;
  {
    let broadcast = Subject::new();
    broadcast.clone().subscribe(|v| i = *v * 2);
    broadcast.next(1);
  }
  assert_eq!(i, 2);
}
