// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 纯单子系统核心：Op（错误+日志单子）、StateOp（状态单子）、IO（IO 单子）。
//!
//! 纯内核实现，零外部依赖。

#[derive(Debug)]
pub struct Op<T> {
    value: Option<T>,
    error: Option<String>,
    logs: Vec<String>,
}

impl<T> Op<T> {
    pub fn pure(value: T) -> Self {
        Self {
            value: Some(value),
            error: None,
            logs: Vec::new(),
        }
    }

    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            value: None,
            error: Some(error.into()),
            logs: Vec::new(),
        }
    }

    pub fn log(mut self, msg: impl Into<String>) -> Self {
        self.logs.push(msg.into());
        self
    }

    pub fn is_ok(&self) -> bool {
        self.value.is_some() && self.error.is_none()
    }

    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }

    pub fn unwrap(self) -> T {
        self.value.expect("Op was in error state")
    }

    pub fn unwrap_err(self) -> String {
        self.error.expect("Op was in ok state")
    }

    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Op<U> {
        match self.value {
            Some(v) => Op {
                value: Some(f(v)),
                error: self.error,
                logs: self.logs,
            },
            None => Op {
                value: None,
                error: self.error,
                logs: self.logs,
            },
        }
    }

    pub fn bind<U, F: FnOnce(T) -> Op<U>>(self, f: F) -> Op<U> {
        match self.value {
            Some(v) => {
                let mut result = f(v);
                let mut logs = self.logs;
                logs.append(&mut result.logs);
                result.logs = logs;
                result
            }
            None => Op {
                value: None,
                error: self.error,
                logs: self.logs,
            },
        }
    }
}

pub struct StateOp<S, A> {
    run: Box<dyn FnOnce(S) -> (A, S)>,
}

impl<S: 'static, A: 'static> StateOp<S, A> {
    pub fn new<F: FnOnce(S) -> (A, S) + 'static>(f: F) -> Self {
        Self { run: Box::new(f) }
    }

    pub fn pure(a: A) -> Self
    where
        A: Clone,
    {
        Self::new(move |s| (a.clone(), s))
    }

    pub fn bind<B: 'static, F: FnOnce(A) -> StateOp<S, B> + 'static>(self, f: F) -> StateOp<S, B> {
        StateOp::new(move |s| {
            let (a, s1) = (self.run)(s);
            (f(a).run)(s1)
        })
    }

    pub fn map<B: 'static, F: FnOnce(A) -> B + 'static>(self, f: F) -> StateOp<S, B> {
        StateOp::new(move |s| {
            let (a, s1) = (self.run)(s);
            (f(a), s1)
        })
    }

    pub fn run(self, initial: S) -> (A, S) {
        (self.run)(initial)
    }

    pub fn eval(self, initial: S) -> A {
        self.run(initial).0
    }

    pub fn exec(self, initial: S) -> S {
        self.run(initial).1
    }
}

impl<S: 'static> StateOp<S, S> {
    pub fn get() -> Self
    where
        S: Clone,
    {
        Self::new(|s| (s.clone(), s))
    }
}

impl<S: 'static> StateOp<S, ()> {
    pub fn put(new_state: S) -> Self {
        Self::new(move |_| ((), new_state))
    }

    pub fn modify<F: FnOnce(S) -> S + 'static>(f: F) -> Self {
        Self::new(move |s| ((), f(s)))
    }
}

pub struct IO<A> {
    perform: Box<dyn FnOnce() -> A>,
}

impl<A: 'static> IO<A> {
    pub fn new<F: FnOnce() -> A + 'static>(f: F) -> Self {
        Self {
            perform: Box::new(f),
        }
    }

    pub fn pure(a: A) -> Self {
        Self::new(move || a)
    }

    pub fn bind<B: 'static, F: FnOnce(A) -> IO<B> + 'static>(self, f: F) -> IO<B> {
        IO::new(move || {
            let a = (self.perform)();
            (f(a).perform)()
        })
    }

    pub fn map<B: 'static, F: FnOnce(A) -> B + 'static>(self, f: F) -> IO<B> {
        IO::new(move || f((self.perform)()))
    }

    pub fn run(self) -> A {
        (self.perform)()
    }
}
