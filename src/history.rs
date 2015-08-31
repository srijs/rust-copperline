use std::collections::VecDeque;
use std::ops::Index;

pub struct Cursor<'a> {
    history: &'a History,
    cur: Option<usize>
}

impl<'a> Cursor<'a> {

    pub fn new(h: &'a History) -> Cursor<'a> {
        Cursor { history: h, cur: None }
    }

    pub fn is_void(&self) -> bool {
        self.cur.is_none()
    }

    pub fn incr(&mut self) -> Cursor<'a> {
        Cursor {
            history: self.history,
            cur: match self.cur {
                Some(i) if i + 1 < self.history.len() => Some(i + 1),
                Some(i) => Some(i),
                None if self.history.len() > 0 => Some(0),
                None => None
            }
        }
    }

    pub fn decr(&mut self) -> Cursor<'a> {
        Cursor {
            history: self.history,
            cur: match self.cur {
                Some(i) if i > 0 => Some(i - 1),
                Some(_) => None,
                None => None
            }
        }
    }

    pub fn get(&self) -> Option<&'a String> {
        match self.cur {
            None => None,
            Some(i) => self.history.get(i)
        }
    }

}

pub struct History {
    deque: VecDeque<String>
}

impl History {

    pub fn new() -> History {
        History {
            deque: VecDeque::new()
        }
    }

    pub fn len(&self) -> usize {
        self.deque.len()
    }

    pub fn push(&mut self, s: String) {
        if s.len() > 0 && self.deque.front() != Option::Some(&s) {
            self.deque.push_front(s)
        }
    }

    pub fn pop(&mut self) -> Option<String> {
        self.deque.pop_front()
    }

    pub fn get(&self, idx: usize) -> Option<&String> {
        let len = self.len();
        if len > 0 && idx < len {
            Some(self.deque.index(idx))
        } else {
            None
        }
    }

}
