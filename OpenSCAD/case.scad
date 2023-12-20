$fn = 30;
bolt_delta_x = 44.45;
bolt_delta_y = 27.94;
bolt_diameter = 7.0;
vented = false;

module usb_hole()
{
	translate([ 0.0, 0.0, -1.0 ])
	minkowski()
	{
		cube([ 11.0, 6.0, 4.0 ], center = true);
		cylinder(h = 2.0, r = 0.5);
	}
}

module case ()
{
	difference()
	{
		union()
		{
			translate([ -0.0, 0.0, 13.0 ])
			minkowski()
			{
				cube([ 52.0, 36.0, 26.0 ], center = true);
				cylinder(h = 4.0, r = 2.0);
			};
		}
		translate([ -26.5, -18.5, 2.0 ])
		cube([ 53.0, 37.0, 30.0 ]);

		translate([ -26.0, 0.0, 30.0 - 11.0 ])
		rotate([ 90.0, 0.0, 90.0 ])
		usb_hole();

		translate([ bolt_delta_x / 2.0, bolt_delta_y / 2.0, -1 ])
		cylinder($fn = 6, h = 4.0, d = 6.0);
		translate([ -bolt_delta_x / 2.0, bolt_delta_y / 2.0, -1 ])
		cylinder($fn = 6, h = 4.0, d = 6.0);
		translate([ bolt_delta_x / 2.0, -bolt_delta_y / 2.0, -1 ])
		cylinder($fn = 6, h = 4.0, d = 6.0);
		translate([ -bolt_delta_x / 2.0, -bolt_delta_y / 2.0, -1 ])
		cylinder($fn = 6, h = 4.0, d = 6.0);

		translate([ 0.0, 0.0, -1.0 ])
		cylinder(h = 4.0, d = 4.0);

		if (vented)
		{
			for (x = [-15:5:15])
				for (z = [6:5:21])
				{
					translate([ x, 30, z ])
					rotate([ 90, 0, 0 ])
					cylinder(r = 1.25, h = 60.0);
				}
			for (y = [-7.5:5:7.5])
				for (z = [6:5:21])
				{
					translate([ 20, y, z ])
					rotate([ 0, 90, 0 ])
					cylinder(r = 1.25, h = 20.0);
				}
			for (y = [-7.5:5:7.5])
				for (z = [6:5:11])
				{
					translate([ -30, y, z ])
					rotate([ 0, 90, 0 ])
					cylinder(r = 1.25, h = 20.0);
				}
		}
	}
	difference()
	{
		union()
		{
			translate([ bolt_delta_x / 2.0, bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 19.5, d = bolt_diameter);
			translate([ bolt_delta_x / 2.0, bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 4.0, d1 = bolt_diameter + 4.0, d2 = bolt_diameter);
			translate([ bolt_delta_x / 2.0 + 2.0, bolt_delta_y / 2.0, 2.0 + 8.75 ])
			cube([ 7.0, 2.0, 19.5 ], center = true);
			translate([ bolt_delta_x / 2.0, bolt_delta_y / 2.0 + 2.0, 2.0 + 8.75 ])
			cube([ 2.0, 7.0, 19.5 ], center = true);

			translate([ -bolt_delta_x / 2.0, bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 19.5, d = bolt_diameter);
			translate([ -bolt_delta_x / 2.0, bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 4.0, d1 = bolt_diameter + 4.0, d2 = bolt_diameter);
			translate([ -bolt_delta_x / 2.0 - 2.0, bolt_delta_y / 2.0, 2.0 + 8.75 ])
			cube([ 7.0, 2.0, 19.5 ], center = true);
			translate([ -bolt_delta_x / 2.0, bolt_delta_y / 2.0 + 2.0, 2.0 + 8.75 ])
			cube([ 2.0, 7.0, 19.5 ], center = true);

			translate([ bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 19.5, d = bolt_diameter);
			translate([ bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 4.0, d1 = bolt_diameter + 4.0, d2 = bolt_diameter);
			translate([ bolt_delta_x / 2.0 + 2.0, -bolt_delta_y / 2.0, 2.0 + 8.75 ])
			cube([ 7.0, 2.0, 19.5 ], center = true);
			translate([ bolt_delta_x / 2.0, -bolt_delta_y / 2.0 - 2.0, 2.0 + 8.75 ])
			cube([ 2.0, 7.0, 19.5 ], center = true);

			translate([ -bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 19.5, d = bolt_diameter);
			translate([ -bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 2.0 ])
			cylinder(h = 4.0, d1 = bolt_diameter + 4.0, d2 = bolt_diameter);
			translate([ -bolt_delta_x / 2.0 - 2.0, -bolt_delta_y / 2.0, 2.0 + 8.75 ])
			cube([ 7.0, 2.0, 19.5 ], center = true);
			translate([ -bolt_delta_x / 2.0, -bolt_delta_y / 2.0 - 2.0, 2.0 + 8.75 ])
			cube([ 2.0, 7.0, 19.5 ], center = true);
		};
		translate([ bolt_delta_x / 2.0, bolt_delta_y / 2.0, 1.0 ])
		cylinder(h = 23.0, d = 2.8);
		translate([ -bolt_delta_x / 2.0, bolt_delta_y / 2.0, 1.0 ])
		cylinder(h = 23.0, d = 2.8);
		translate([ bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 1.0 ])
		cylinder(h = 23.0, d = 2.8);
		translate([ -bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 1.0 ])
		cylinder(h = 23.0, d = 2.8);
	}
}

module baseplate()
{
	difference()
	{
		union()
		{
			translate([ -26.0, -18.0, 0 ])
			minkowski()
			{
				cube([ 52.0, 36.0, 1.0 ]);
				cylinder(h = 1.0, r = 2.0);
			};
			translate([ bolt_delta_x / 2.0, bolt_delta_y / 2.0, 0 ])
			cylinder(h = 8.5, d1 = bolt_diameter + 1.0, d2 = bolt_diameter);
			translate([ -bolt_delta_x / 2.0, bolt_delta_y / 2.0, 0 ])
			cylinder(h = 8.5, d1 = bolt_diameter + 1.0, d2 = bolt_diameter);
			translate([ bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 0 ])
			cylinder(h = 8.5, d1 = bolt_diameter + 1.0, d2 = bolt_diameter);
			translate([ -bolt_delta_x / 2.0, -bolt_delta_y / 2.0, 0 ])
			cylinder(h = 8.5, d1 = bolt_diameter + 1.0, d2 = bolt_diameter);
		}
		translate([ 26.0 - 8.8, 0, -1 ])
		cylinder(h = 9.5, d = 8.2);
		translate([ 26.0 - 50.0 + 8.2, -6.0, -1.0 ])
		cube([ 25.0, 12.0, 4.0 ]);
		translate([ bolt_delta_x / 2, bolt_delta_y / 2, -1 ])
		cylinder(h = 11.5, d = 2.8);
		translate([ -bolt_delta_x / 2, bolt_delta_y / 2, -1 ])
		cylinder(h = 11.5, d = 2.8);
		translate([ bolt_delta_x / 2, -bolt_delta_y / 2, -1 ])
		cylinder(h = 11.5, d = 2.8);
		translate([ -bolt_delta_x / 2, -bolt_delta_y / 2, -1 ])
		cylinder(h = 11.5, d = 2.8);
		translate([ bolt_delta_x / 2, bolt_delta_y / 2, -1 ])
		cylinder(h = 3.0, d = 5.0);
		translate([ -bolt_delta_x / 2, bolt_delta_y / 2, -1 ])
		cylinder(h = 3.0, d = 5.0);
		translate([ bolt_delta_x / 2, -bolt_delta_y / 2, -1 ])
		cylinder(h = 3.0, d = 5.0);
		translate([ -bolt_delta_x / 2, -bolt_delta_y / 2, -1 ])
		cylinder(h = 3.0, d = 5.0);
	}
}

module pipe(h, do, di)
{
	difference()
	{
		cylinder(h = h, d = do);
		translate([ 0.0, 0.0, -1.0 ])
		cylinder(h = h + 2.0, d = di);
	}
}

module knob()
{
	difference()
	{
		union()
		{
			difference()
			{
				minkowski()
				{
					cylinder(h = 8.0, d1 = 9.0, d2 = 8.0);
					sphere(r = 2.0);
				};
				translate([ 0.0, 0.0, -1.0 ])
				cylinder(h = 9.0, d = 6);
				translate([ 0.0, 0.0, -2.5 ])
				cube([ 20.0, 20.0, 5.0 ], center = true);
			}
			translate([ -4.0, 1.5, 0.0 ])
			cube([ 8.0, 2.0, 8.0 ]);
            translate([ 0.0, 0.0, -2.0 ]) pipe(2.0, 13.0, 9.0);
		}
		translate([ 0.0, 0.0, -1.0 ])
		pipe(h = 9.0, do = 9.0, di = 7.3);
		translate([ -2.95, 2.1, 0.0 ])
		cube([ 5.9, 1.3, 8.0 ]);
		translate([ -1.5, 2.5, 0.0 ])
		cube([ 3, 1.4, 8.0 ]);
		translate([ -0.8, 1.4, 0.0 ])
		cube([ 1.6, 2.0, 8.0 ]);

		for (a = [0.0:40.0:360.0])
		{
			translate([ 6.5 * sin(a), 6.5 * cos(a), -1.0 ])
			cylinder(h = 12.0, d1 = 2.4, d2 = 2.0);
		}
	}
}
// baseplate();
// case ();
knob();
// pipe(h=20, do=20, di=16);
