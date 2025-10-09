pub fn print_table(headers: Vec<&str>, rows: Vec<Vec<String>>) {
    if rows.is_empty() {
        println!("No items to display.");
        return;
    }

    let mut column_widths = headers.iter().map(|h| h.len()).collect::<Vec<_>>();

    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if cell.len() > column_widths[i] {
                column_widths[i] = cell.len();
            }
        }
    }

    // Print header
    for (i, header) in headers.iter().enumerate() {
        print!("{:<width$}  ", header, width = column_widths[i]);
    }
    println!();

    // Print separator
    for width in &column_widths {
        print!("{:-<width$}  ", "", width = width);
    }
    println!();

    // Print rows
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            print!("{:<width$}  ", cell, width = column_widths[i]);
        }
        println!();
    }
}