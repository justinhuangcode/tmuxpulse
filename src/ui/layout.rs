/// Calculate grid dimensions for session cards.
/// Returns (columns, rows).
pub fn calculate_grid(
    area_width: u16,
    area_height: u16,
    item_count: usize,
    min_card_width: u16,
    _min_card_height: u16,
) -> (usize, usize) {
    if item_count == 0 || area_width == 0 || area_height == 0 {
        return (0, 0);
    }

    let max_cols = (area_width / min_card_width.max(1)) as usize;
    let cols = max_cols.max(1).min(item_count);

    let rows = item_count.div_ceil(cols);

    (cols, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_empty() {
        assert_eq!(calculate_grid(100, 50, 0, 40, 12), (0, 0));
    }

    #[test]
    fn grid_single() {
        assert_eq!(calculate_grid(100, 50, 1, 40, 12), (1, 1));
    }

    #[test]
    fn grid_fits_two_columns() {
        let (cols, rows) = calculate_grid(100, 50, 4, 40, 12);
        assert_eq!(cols, 2);
        assert_eq!(rows, 2);
    }

    #[test]
    fn grid_narrow_terminal() {
        let (cols, rows) = calculate_grid(30, 50, 4, 40, 12);
        // 30 / 40 = 0, but min 1 column
        assert_eq!(cols, 1);
        assert_eq!(rows, 4);
    }

    #[test]
    fn grid_many_sessions() {
        let (cols, rows) = calculate_grid(160, 50, 10, 40, 12);
        assert_eq!(cols, 4);
        assert_eq!(rows, 3); // ceil(10/4)
    }
}
