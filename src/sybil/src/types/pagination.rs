use candid::CandidType;
use paginate::Pages;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, CandidType)]
pub struct Pagination {
    pub page: usize,
    pub size: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, CandidType)]
pub struct PaginationResult<T: Clone> {
    pub page: usize,
    pub size: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub items: Vec<T>,
}

impl<T: Clone> From<Vec<T>> for PaginationResult<T> {
    fn from(data: Vec<T>) -> Self {
        PaginationResult {
            page: 1,
            size: data.len(),
            total_items: data.len(),
            total_pages: 1,
            items: data,
        }
    }
}

impl Pagination {
    pub fn paginate<T: Clone>(&self, data: Vec<T>) -> PaginationResult<T> {
        let total_items = data.len();
        let items_per_page = self.size;
        let pages = Pages::new(total_items, items_per_page);
        let page = pages.with_offset(self.page - 1);

        let items = if self.page > pages.page_count() {
            vec![]
        } else {
            data[page.start..page.end + 1].to_vec()
        };

        PaginationResult {
            page: self.page,
            size: self.size,
            total_items,
            total_pages: pages.page_count(),
            items,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pagination_test() {
        let vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let pagination = Pagination { page: 1, size: 5 };
        let res = pagination.paginate(vec.clone());

        assert_eq!(res.page, 1);
        assert_eq!(res.size, 5);
        assert_eq!(res.total_items, 16);
        assert_eq!(res.total_pages, 4);
        assert_eq!(vec[0..5], res.items[..]);

        let pagination = Pagination { page: 4, size: 5 };
        let res = pagination.paginate(vec.clone());

        assert_eq!(res.page, 4);
        assert_eq!(res.size, 5);
        assert_eq!(res.total_items, 16);
        assert_eq!(res.total_pages, 4);
        assert_eq!(vec[15..16], res.items[..]);

        let pagination = Pagination { page: 10, size: 5 };
        let res = pagination.paginate(vec.clone());

        assert_eq!(res.page, 10);
        assert_eq!(res.size, 5);
        assert_eq!(res.total_items, 16);
        assert_eq!(res.total_pages, 4);
        assert!(res.items.is_empty());

        let pagination = Pagination { page: 2, size: 1 };
        let res = pagination.paginate(vec.clone());

        assert_eq!(res.page, 2);
        assert_eq!(res.size, 1);
        assert_eq!(res.total_items, 16);
        assert_eq!(res.total_pages, 16);
        assert_eq!(vec[1..2], res.items);
    }
}
