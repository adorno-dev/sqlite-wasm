//! Test data fixtures

#[allow(unused)]
use js_sys::Array;
use wasm_bindgen::JsValue;

/// SQL statements for testing
pub mod sql {
    pub const CREATE_USERS: &str = r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER,
            salary REAL,
            is_active INTEGER DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    "#;

    pub const CREATE_PRODUCTS: &str = r#"
        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price REAL,
            stock INTEGER DEFAULT 0,
            category TEXT
        )
    "#;

    pub const CREATE_ORDERS: &str = r#"
        CREATE TABLE IF NOT EXISTS orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            product_id INTEGER NOT NULL,
            quantity INTEGER NOT NULL,
            order_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (product_id) REFERENCES products(id)
        )
    "#;

    pub const INSERT_USER: &str = "INSERT INTO users (name, email, age, salary, is_active) VALUES (?, ?, ?, ?, ?)";
    pub const INSERT_PRODUCT: &str = "INSERT INTO products (name, price, stock, category) VALUES (?, ?, ?, ?)";
    pub const INSERT_ORDER: &str = "INSERT INTO orders (user_id, product_id, quantity) VALUES (?, ?, ?)";
    pub const SELECT_ALL_USERS: &str = "SELECT * FROM users ORDER BY id";
    pub const SELECT_USER_BY_ID: &str = "SELECT * FROM users WHERE id = ?";
    pub const UPDATE_USER_AGE: &str = "UPDATE users SET age = ? WHERE id = ?";
    pub const DELETE_USER: &str = "DELETE FROM users WHERE id = ?";
}

/// Test data
pub mod data {
    use super::*;

    pub fn user_rows() -> Vec<Vec<JsValue>> {
        vec![
            vec![
                JsValue::from_str("Alice"),
                JsValue::from_str("alice@test.com"),
                JsValue::from(30),
                JsValue::from(75000.0),
                JsValue::from(1),
            ],
            vec![
                JsValue::from_str("Bob"),
                JsValue::from_str("bob@test.com"),
                JsValue::from(25),
                JsValue::from(65000.0),
                JsValue::from(1),
            ],
            vec![
                JsValue::from_str("Charlie"),
                JsValue::from_str("charlie@test.com"),
                JsValue::from(35),
                JsValue::from(85000.0),
                JsValue::from(1),
            ],
        ]
    }

    pub fn product_rows() -> Vec<Vec<JsValue>> {
        vec![
            vec![
                JsValue::from_str("Laptop"),
                JsValue::from(1299.99),
                JsValue::from(10),
                JsValue::from_str("Electronics"),
            ],
            vec![
                JsValue::from_str("Mouse"),
                JsValue::from(29.99),
                JsValue::from(50),
                JsValue::from_str("Electronics"),
            ],
            vec![
                JsValue::from_str("Keyboard"),
                JsValue::from(89.99),
                JsValue::from(30),
                JsValue::from_str("Electronics"),
            ],
        ]
    }

    pub fn large_user_batch() -> Vec<Vec<JsValue>> {
        let mut batch = Vec::with_capacity(100);
        for i in 0..100 {
            batch.push(vec![
                JsValue::from_str(&format!("User {}", i)),
                JsValue::from_str(&format!("user{}@test.com", i)),
                JsValue::from(20 + (i % 50)),
                JsValue::from(50000.0 + (i * 1000) as f64),
                JsValue::from(i % 2),
            ]);
        }
        batch
    }
}