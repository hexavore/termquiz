---
title: "Midterm Exam: Systems Programming"
start: 2026-01-02T10:00:00-05:00
end: 2026-02-12T12:00:00-05:00

acknowledgment:
  required: true
  text: |
    I affirm that I will complete this exam without assistance from
    any other person or unauthorized resource. I understand that any
    violation of academic integrity will result in disciplinary action.
---

# Midterm Exam: Systems Programming

Welcome to the midterm. Read all questions carefully before starting.

---

## 1. Multiple Choice (Single)

Which system call creates a new process?

- [ ] exec
- [x] fork
- [ ] spawn
- [ ] clone

---

## 2. Multiple Choice (Multi)

Select all valid Rust integer types:

- [x] i32
- [x] u64
- [ ] int
- [x] usize
- [ ] long

---

## 3. Short Answer

What flag makes `grep` case-insensitive?

> short

---

## 4. Long Answer

Explain how the borrow checker prevents data races at compile time.

> long

:::hint
Consider what happens when multiple references exist.
:::

:::hint
Think about mutable vs immutable borrows.
:::

---

## 5. File Submission

Submit your implementation of the linked list.

> file(max_files: 3, max_size: 5MB, accept: .rs)

:::hint
Remember to handle the empty list case.
:::

---
