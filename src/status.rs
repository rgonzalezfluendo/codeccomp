#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Status {
    pub zoom: usize,
    pub offset_x: i32,
    pub offset_y: i32,
    pub border: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub xpos: i32,
    pub ypos: i32,
    pub width: i32,
    pub height: i32,
    pub crop_right: i32,
    pub crop_left: i32,
}

const BORDER_STEP: usize = 10;
const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
const HALF_WIDTH: i32 = WIDTH / 2;

impl Default for Status {
    fn default() -> Self {
        Self {
            zoom: 100,
            offset_x: 0,
            offset_y: 0,
            border: HALF_WIDTH,
            width: WIDTH,
            height: HEIGHT,
        }
    }
}

impl Status {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            border: width / 2,
            ..Default::default()
        }
    }

    /// Reset default values
    pub fn reset(&mut self) {
        let d = Status::default();
        self.zoom = d.zoom;
        self.offset_x = d.offset_x;
        self.offset_y = d.offset_y;
        self.border = d.border;
    }

    /// Reset only border to default values
    pub fn reset_border(&mut self) {
        let d = Status::default();
        self.border = d.border;
    }

    /// Reset only border to default values
    pub fn reset_position(&mut self) {
        let d = Status::default();
        self.zoom = d.zoom;
        self.offset_x = d.offset_x;
        self.offset_y = d.offset_y;
    }

    /// Moves the viewport by `x_step` pixels horizontally and `y_step` pixels vertically.
    /// Does not clamp the values, allowing offsets to exceed valid bounds.
    pub fn move_pos(&mut self, x_step: i32, y_step: i32) {
        self.offset_x += x_step;
        self.offset_y += y_step;
    }

    /// Set offset_x and offset_y
    pub fn move_pos_to(&mut self, x: i32, y: i32) {
        self.offset_x = x;
        self.offset_y = y;
    }

    /// Offsets the border inside the bounds.
    pub fn move_border(&mut self, offset: i32) {
        self.move_border_to(self.border + offset);
    }

    /// Set border position inside the bounds.
    pub fn move_border_to(&mut self, new_border: i32) {
        if new_border < 0 {
            self.border = 0
        } else if new_border > self.width {
            self.border = self.width
        } else {
            self.border = new_border
        }
    }

    /// Increases the zoom level, capping it at a sensible maximum (e.g., 1000000)
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom + BORDER_STEP).min(1000000);
    }

    /// Decreases the zoom level, ensuring it stays at a minimum of 1
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom.saturating_sub(BORDER_STEP)).max(1);
    }

    /// Increases the zoom level, capping it at a sensible maximum (e.g., 1000000)
    /// Update offset to keep centered
    pub fn zoom_in_center_at(&mut self, x: i32, y: i32) {
        self.zoom_in();
        self.fix_offset_when_zoom(x, y, true);
    }

    /// Decreases the zoom level, ensuring it stays at a minimum of 1,
    /// Update offset to keep centered
    pub fn zoom_out_center_at(&mut self, x: i32, y: i32) {
        self.zoom_out();
        self.fix_offset_when_zoom(x, y, false);
    }

    fn fix_offset_when_zoom(&mut self, x: i32, y: i32, inside: bool) {
        let diff = x - (self.width / 2);
        let new_offset = diff / (BORDER_STEP as i32);
        if inside {
            self.offset_x -= new_offset;
        } else {
            self.offset_x += new_offset;
        }

        let diff = y - (self.height / 2);
        let new_offset = diff / (BORDER_STEP as i32);
        if inside {
            self.offset_y -= new_offset;
        } else {
            self.offset_y += new_offset;
        }
    }

    /// Calculates the two `Position`s for the input videos based on the status values
    pub fn get_positions(&self) -> (Position, Position) {
        let zoom_factor = (self.zoom as f32) / 100.0;
        let viewport_width = (self.width as f32 * zoom_factor) as i32;
        let viewport_height = (self.height as f32 * zoom_factor) as i32;
        let viewport_offset_x = self.offset_x - (viewport_width - self.width) / 2;
        let viewport_offset_y = self.offset_y - (viewport_height - self.height) / 2;

        let pos0 = Position {
            xpos: if viewport_offset_x < self.border {
                viewport_offset_x
            } else {
                0
            },
            ypos: viewport_offset_y,
            width: {
                if viewport_offset_x < self.border {
                    if self.border - viewport_offset_x > viewport_width {
                        viewport_width
                    } else {
                        self.border - viewport_offset_x
                    }
                } else {
                    0
                }
            },
            height: viewport_height,
            crop_right: {
                if viewport_width + viewport_offset_x < self.border {
                    0
                } else if viewport_offset_x > self.border {
                    self.width
                } else {
                    // Note crop before zoom scaling (because glvideomixer implementation)w
                    let scale = self.width as f32 / viewport_width as f32;
                    let crop_right_scaled =
                        (viewport_width + viewport_offset_x - self.border) as f32;
                    (crop_right_scaled * scale) as i32
                }
            },
            crop_left: 0,
        };

        let pos1 = Position {
            xpos: if viewport_offset_x > self.border {
                viewport_offset_x
            } else {
                self.border
            },
            ypos: viewport_offset_y,
            width: {
                //TODO refactor
                if self.border < (viewport_width - viewport_offset_x) {
                    if viewport_width > self.border - viewport_offset_x {
                        if self.border < viewport_offset_x {
                            viewport_width
                        } else {
                            viewport_width - self.border + viewport_offset_x
                        }
                    } else {
                        0
                    }
                } else if self.border < viewport_offset_x {
                    viewport_width
                } else {
                    viewport_width - self.border + viewport_offset_x
                }
            },
            height: viewport_height,
            crop_right: 0,
            crop_left: {
                if viewport_width + viewport_offset_x < self.border {
                    self.width
                } else if viewport_offset_x > self.border {
                    0
                } else {
                    // Note crop before zoom scaling (because glvideomixer implementation)w
                    let scale = self.width as f32 / viewport_width as f32;
                    let crop_right_scaled = (self.border - viewport_offset_x) as f32;
                    (crop_right_scaled * scale) as i32
                }
            },
        };

        (pos0, pos1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_default() {
        let status = Status::default();

        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);
    }

    #[test]
    fn test_get_positions_default() {
        let status = Status::default();
        let (pos0, pos1) = status.get_positions();

        assert_eq!(pos0.xpos, 0);
        assert_eq!(pos0.ypos, 0);
        assert_eq!(pos0.width, HALF_WIDTH);
        assert_eq!(pos0.height, HEIGHT);
        assert_eq!(pos0.crop_right, HALF_WIDTH);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, HALF_WIDTH);
        assert_eq!(pos1.ypos, 0);
        assert_eq!(pos1.width, HALF_WIDTH);
        assert_eq!(pos1.height, HEIGHT);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, HALF_WIDTH);
    }

    #[test]
    fn test_move_pos_left() {
        let mut status = Status::default();
        status.move_pos(-10, 0);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, -10);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, -10);
        assert_eq!(pos0.ypos, 0);
        assert_eq!(pos0.width, HALF_WIDTH + 10);
        assert_eq!(pos0.height, HEIGHT);
        assert_eq!(pos0.crop_right, HALF_WIDTH - 10);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, HALF_WIDTH);
        assert_eq!(pos1.ypos, 0);
        assert_eq!(pos1.width, HALF_WIDTH - 10);
        assert_eq!(pos1.height, HEIGHT);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, HALF_WIDTH + 10);

        status.reset_position();
        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);
    }

    #[test]
    fn test_move_pos_left_out_of_border() {
        let mut status = Status::default();
        status.move_pos(-1000, 0);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, -1000);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, -1000);
        assert_eq!(pos0.ypos, 0);
        assert_eq!(pos0.width, WIDTH);
        assert_eq!(pos0.height, HEIGHT);
        assert_eq!(pos0.crop_right, 0);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, HALF_WIDTH);
        assert_eq!(pos1.ypos, 0);
        assert_eq!(pos1.width, 0);
        assert_eq!(pos1.height, HEIGHT);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, WIDTH);
    }

    #[test]
    fn test_move_pos_right_out_of_border() {
        let mut status = Status::default();
        status.move_pos(1000, 0);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 1000);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, 0);
        assert_eq!(pos0.ypos, 0);
        assert_eq!(pos0.width, 0);
        assert_eq!(pos0.height, HEIGHT);
        assert_eq!(pos0.crop_right, WIDTH);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, 1000);
        assert_eq!(pos1.ypos, 0);
        assert_eq!(pos1.width, WIDTH);
        assert_eq!(pos1.height, HEIGHT);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, 0);
    }

    #[test]
    fn test_move_border_left() {
        let mut status = Status::default();
        status.move_border(10);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH + 10);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, 0);
        assert_eq!(pos0.ypos, 0);
        assert_eq!(pos0.width, HALF_WIDTH + 10);
        assert_eq!(pos0.height, HEIGHT);
        assert_eq!(pos0.crop_right, HALF_WIDTH - 10);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, HALF_WIDTH + 10);
        assert_eq!(pos1.ypos, 0);
        assert_eq!(pos1.width, HALF_WIDTH - 10);
        assert_eq!(pos1.height, HEIGHT);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, HALF_WIDTH + 10);

        status.reset_position();
        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH + 10);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);
    }

    #[test]
    fn test_zoom_in() {
        let mut status = Status::default();
        status.zoom_in();
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 110);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, -64);
        assert_eq!(pos0.ypos, -36);
        assert_eq!(pos0.width, 704);
        assert_eq!(pos0.height, 792);
        assert_eq!(pos0.crop_right, 640); // Note crop before zoom scaling (because glvideomixer implementation)
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, 640);
        assert_eq!(pos1.ypos, -36);
        assert_eq!(pos1.width, 704);
        assert_eq!(pos1.height, 792);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, 640); // Note crop before zoom scaling (because glvideomixer implementation)
    }

    #[test]
    fn test_zoom_out_five_times() {
        let mut status = Status::default();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, 384);
        assert_eq!(pos0.ypos, 216);
        assert_eq!(pos0.width, 256);
        assert_eq!(pos0.height, 288);
        assert_eq!(pos0.crop_right, 640); // Note crop before zoom scaling (because glvideomixer implementation)
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, 640);
        assert_eq!(pos1.ypos, 216);
        assert_eq!(pos1.width, 256);
        assert_eq!(pos1.height, 288);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, 640);
    }

    #[test]
    fn test_zoom_out_five_times_and_move() {
        let mut status = Status::default();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, 384);
        assert_eq!(pos0.ypos, 216);
        assert_eq!(pos0.width, 256);
        assert_eq!(pos0.height, 288);
        assert_eq!(pos0.crop_right, 640);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, 640);
        assert_eq!(pos1.ypos, 216);
        assert_eq!(pos1.width, 256);
        assert_eq!(pos1.height, 288);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, 640);

        let current_width = pos0.width + pos1.width;
        status.move_pos(-10, 0);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, -10);
        assert_eq!(pos0.width + pos1.width, current_width);
        assert_eq!(pos0.crop_right, 615);
        assert_eq!(pos1.crop_left, 665);

        status.move_pos(-10, 0);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, -20);
        assert_eq!(pos0.width + pos1.width, current_width);
        assert_eq!(pos0.crop_right, 590);
        assert_eq!(pos1.crop_left, 690);

        status.move_pos(-10, 0);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, -30);
        assert_eq!(pos0.width + pos1.width, current_width);
        assert_eq!(pos0.crop_right, 565);
        assert_eq!(pos1.crop_left, 715);

        status.move_pos(-10, 0);
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, -40);
        assert_eq!(pos0.width + pos1.width, current_width);
        assert_eq!(pos0.crop_right, 540);
        assert_eq!(pos1.crop_left, 740);
    }

    #[test]
    fn test_zoom_out_five_times_only_one_video() {
        let mut status = Status::default();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        status.zoom_out();
        let (pos0, pos1) = status.get_positions();

        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, 384);
        assert_eq!(pos0.ypos, 216);
        assert_eq!(pos0.width, 256);
        assert_eq!(pos0.height, 288);
        assert_eq!(pos0.crop_right, 640);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, 640);
        assert_eq!(pos1.ypos, 216);
        assert_eq!(pos1.width, 256);
        assert_eq!(pos1.height, 288);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, 640);

        status.move_border_to(0);
        let (pos0, pos1) = status.get_positions();
        assert_eq!(status.zoom, 40);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, 0);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        assert_eq!(pos0.xpos, 0);
        assert_eq!(pos0.ypos, 216);
        assert_eq!(pos0.width, 0);
        assert_eq!(pos0.height, 288);
        assert_eq!(pos0.crop_right, 1280);
        assert_eq!(pos0.crop_left, 0);

        assert_eq!(pos1.xpos, 384);
        assert_eq!(pos1.ypos, 216);
        assert_eq!(pos1.width, 512);
        assert_eq!(pos1.height, 288);
        assert_eq!(pos1.crop_right, 0);
        assert_eq!(pos1.crop_left, 0);
    }

    #[test]
    fn test_zoom_inout_center_at() {
        let mut status = Status::default();
        status.zoom_in_center_at(0, 0);

        assert_eq!(status.zoom, 110);
        assert_eq!(status.offset_x, 64);
        assert_eq!(status.offset_y, 36);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        status.zoom_out_center_at(0, 0);
        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);

        status.zoom_out_center_at(0, 0);
        assert_eq!(status.zoom, 90);
        assert_eq!(status.offset_x, -64);
        assert_eq!(status.offset_y, -36);
        assert_eq!(status.border, HALF_WIDTH);
        assert_eq!(status.width, WIDTH);
        assert_eq!(status.height, HEIGHT);
    }

    #[test]
    fn test_new_zoom_inout_center_at() {
        let width = 12800;
        let height = 7200;
        let half_width = 12800 / 2;

        let mut status = Status::new(width, height);
        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, half_width);
        assert_eq!(status.width, width);
        assert_eq!(status.height, height);

        status.zoom_in_center_at(0, 0);

        assert_eq!(status.zoom, 110);
        assert_eq!(status.offset_x, 640);
        assert_eq!(status.offset_y, 360);
        assert_eq!(status.border, half_width);
        assert_eq!(status.width, width);
        assert_eq!(status.height, height);

        status.zoom_out_center_at(0, 0);
        assert_eq!(status.zoom, 100);
        assert_eq!(status.offset_x, 0);
        assert_eq!(status.offset_y, 0);
        assert_eq!(status.border, half_width);
        assert_eq!(status.width, width);
        assert_eq!(status.height, height);

        status.zoom_out_center_at(0, 0);
        assert_eq!(status.zoom, 90);
        assert_eq!(status.offset_x, -640);
        assert_eq!(status.offset_y, -360);
        assert_eq!(status.border, half_width);
        assert_eq!(status.width, width);
        assert_eq!(status.height, height);
    }
}
