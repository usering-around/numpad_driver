use evdev_rs::enums::EV_KEY;

pub struct RowItem<T> {
    left_x: usize,
    right_x: usize,
    item: T,
}
pub struct Row<T> {
    items: Vec<RowItem<T>>,
    max_y: usize,
    min_y: usize,
}
pub struct Layout<T> {
    rows: Vec<Row<T>>,
}

impl<T: Clone> Layout<T> {
    pub fn get_item(&self, x: usize, y: usize) -> Option<T> {
        for row in self.rows.iter() {
            if row.min_y <= y && y <= row.max_y {
                for item in row.items.iter() {
                    if item.left_x <= x && x <= item.right_x {
                        return Some(item.item.clone());
                    }
                }
            }
        }
        None
    }
}

pub fn default_numpad_layout() -> Layout<EV_KEY> {
    fn insert_next_key(vec: &mut Vec<RowItem<EV_KEY>>, right_x: usize, key: EV_KEY) {
        let margin_x = 50;
        vec.push(RowItem {
            left_x: vec.last().unwrap().right_x + margin_x,
            right_x,
            item: key,
        });
    }
    fn insert_next_row(vec: &mut Vec<Row<EV_KEY>>, items: Vec<RowItem<EV_KEY>>) {
        let margin_y = 100;
        vec.push(Row {
            items,
            max_y: vec.last().unwrap().max_y + margin_y + vec.last().unwrap().max_y
                - vec.last().unwrap().min_y,
            min_y: vec.last().unwrap().max_y + margin_y,
        });
    }
    let mut rows = Vec::new();
    let mut items = vec![RowItem {
        left_x: 330,
        right_x: 860,
        item: EV_KEY::KEY_7,
    }];
    let items_ref = &mut items;
    insert_next_key(items_ref, 1600, EV_KEY::KEY_8);
    insert_next_key(items_ref, 2260, EV_KEY::KEY_9);
    insert_next_key(items_ref, 3030, EV_KEY::KEY_SLASH);
    insert_next_key(items_ref, 3750, EV_KEY::KEY_NUMLOCK);

    let first_row = Row {
        items,
        min_y: 200,
        max_y: 680,
    };
    rows.push(first_row);

    let mut items = vec![RowItem {
        left_x: 330,
        right_x: 860,
        item: EV_KEY::KEY_4,
    }];
    let items_ref = &mut items;
    insert_next_key(items_ref, 1600, EV_KEY::KEY_5);
    insert_next_key(items_ref, 2260, EV_KEY::KEY_6);
    insert_next_key(items_ref, 3030, EV_KEY::KEY_KPASTERISK);
    insert_next_key(items_ref, 3750, EV_KEY::KEY_BACKSPACE);
    insert_next_row(&mut rows, items);

    let mut items = vec![RowItem {
        left_x: 330,
        right_x: 860,
        item: EV_KEY::KEY_1,
    }];
    let items_ref = &mut items;
    insert_next_key(items_ref, 1600, EV_KEY::KEY_2);
    insert_next_key(items_ref, 2260, EV_KEY::KEY_3);
    insert_next_key(items_ref, 3030, EV_KEY::KEY_MINUS);
    insert_next_key(items_ref, 3750, EV_KEY::KEY_ENTER);
    insert_next_row(&mut rows, items);

    let mut items = vec![RowItem {
        left_x: 860,
        right_x: 1600,
        item: EV_KEY::KEY_0,
    }];
    let items_ref = &mut items;
    insert_next_key(items_ref, 2260, EV_KEY::KEY_DOT);
    insert_next_key(items_ref, 3030, EV_KEY::KEY_KPPLUS);
    insert_next_key(items_ref, 3750, EV_KEY::KEY_ENTER);
    insert_next_row(&mut rows, items);

    Layout { rows }
}
