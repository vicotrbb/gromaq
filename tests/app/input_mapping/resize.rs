use gromaq::app::{NativePtyResize, NativeResizeGridMapper};

#[test]
fn native_resize_grid_mapper_fits_cells_inside_window_padding() {
    let mapper = NativeResizeGridMapper::new(14, 18, 14, 0).unwrap();

    assert_eq!(
        mapper.resize_for_window(1280, 800),
        Some(NativePtyResize {
            cols: 89,
            rows: 42,
            pixel_width: 1280,
            pixel_height: 800,
        })
    );
    assert_eq!(
        mapper.resize_for_window(640, 400),
        Some(NativePtyResize {
            cols: 43,
            rows: 20,
            pixel_width: 640,
            pixel_height: 400,
        })
    );
    assert_eq!(
        mapper.resize_for_window(20, 20),
        Some(NativePtyResize {
            cols: 1,
            rows: 1,
            pixel_width: 20,
            pixel_height: 20,
        })
    );
    assert_eq!(mapper.resize_for_window(0, 400), None);
    assert_eq!(NativeResizeGridMapper::new(0, 18, 14, 0), None);
}

#[test]
fn native_resize_grid_mapper_fits_spaced_cells_inside_window_padding() {
    let mapper = NativeResizeGridMapper::new(14, 18, 14, 2).unwrap();

    assert_eq!(
        mapper.resize_for_window(1280, 800),
        Some(NativePtyResize {
            cols: 78,
            rows: 38,
            pixel_width: 1280,
            pixel_height: 800,
        })
    );
}
