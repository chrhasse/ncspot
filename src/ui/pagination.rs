use std::sync::{Arc, RwLock, LockResult, RwLockReadGuard};

use crate::traits::ListItem;
use rspotify::model::page::Page;

pub type FetchPageFn<I> = dyn Fn(u32) -> Option<Page<I>>;
pub struct ResultPage<I> {
    offset: u32,
    limit: usize,
    total: u32,
    pub items: Arc<RwLock<Vec<I>>>,
    fetch_page: Arc<FetchPageFn<I>>,
}

impl<I> ResultPage<I> {
    pub fn new(
        offset: u32,
        limit: usize,
        fetch_page: Arc<FetchPageFn<I>>,
    ) -> Option<ResultPage<I>> {
        if let Some(first_page) = fetch_page(offset) {
            let result = ResultPage {
                offset,
                limit,
                total: first_page.total,
                items: Arc::new(RwLock::new(first_page.items)),
                fetch_page: fetch_page.clone(),
            };
            Some(result)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.items.read().unwrap().len()
    }

    pub fn at_end(&self) -> bool {
        (self.offset + self.limit as u32) >= self.total
    }

    pub fn next(&mut self) -> bool {
        let offset = self.offset + self.limit as u32;
        if !self.at_end() {
            if let Some(next_page) = (self.fetch_page)(offset) {
                self.offset = offset;
                self.items.write().unwrap().extend(next_page.items);
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}


pub type Paginator<I> = Box<dyn Fn(Arc<RwLock<Vec<I>>>) + Send + Sync>;

pub struct Pagination<I: ListItem> {
    max_content: Arc<RwLock<Option<usize>>>,
    callback: Arc<RwLock<Option<Paginator<I>>>>,
    busy: Arc<RwLock<bool>>,
}

impl<I: ListItem> Default for Pagination<I> {
    fn default() -> Self {
        Pagination {
            max_content: Arc::new(RwLock::new(None)),
            callback: Arc::new(RwLock::new(None)),
            busy: Arc::new(RwLock::new(false)),
        }
    }
}

// TODO: figure out why deriving Clone doesn't work
impl<I: ListItem> Clone for Pagination<I> {
    fn clone(&self) -> Self {
        Pagination {
            max_content: self.max_content.clone(),
            callback: self.callback.clone(),
            busy: self.busy.clone(),
        }
    }
}

impl<I: ListItem> Pagination<I> {
    pub fn clear(&mut self) {
        *self.max_content.write().unwrap() = None;
        *self.callback.write().unwrap() = None;
    }
    pub fn set(&mut self, max_content: usize, callback: Paginator<I>) {
        *self.max_content.write().unwrap() = Some(max_content);
        *self.callback.write().unwrap() = Some(callback);
    }

    pub fn max_content(&self) -> Option<usize> {
        *self.max_content.read().unwrap()
    }

    fn is_busy(&self) -> bool {
        *self.busy.read().unwrap()
    }

    pub fn call(&self, content: &Arc<RwLock<Vec<I>>>) {
        let pagination = self.clone();
        let content = content.clone();
        if !self.is_busy() {
            *self.busy.write().unwrap() = true;
            std::thread::spawn(move || {
                let cb = pagination.callback.read().unwrap();
                if let Some(ref cb) = *cb {
                    debug!("calling paginator!");
                    cb(content);
                    *pagination.busy.write().unwrap() = false;
                }
            });
        }
    }
}
