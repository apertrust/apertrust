# The modules' words in British English, the source language (docs/gui.md, Module panels
# from definitions). Each id follows from the module's definition in apertrust-modules:
#   module-<module>-title, module-<module>-auto, module-<module>-tab-<tab>,
#   module-<module>-group-<group>, module-<module>-collapsible-<group>,
#   module-<module>-control-<control>-label, -full and -promise,
#   module-<module>-control-<control>-option-<option>, a tool's setting's
#   module-<module>-setting-<setting>-label, -full, -promise and -option-<option>, and a
#   preset row's module-<module>-preset-<preset>-label, -full, -promise and
#   -option-<option>.
# Units are unit-<unit>, and a preset row's Custom preset-custom, in ui.ftl. A test in
# crates/modules/tests checks that every id a built-in module implies is here; Ukrainian
# (uk) and Polish (pl) translate each one.

## Raw black/white point (docs/modules/raw-black-white-point.md): darktable's raw
## black/white point, its labels and tooltips in British English.

module-rawprepare-title = Raw black/white point
module-rawprepare-control-raw_black_level_separate_0-label = Black level 0
module-rawprepare-control-raw_black_level_separate_0-full = Black level 0
module-rawprepare-control-raw_black_level_separate_0-promise = Black level 0.
module-rawprepare-control-raw_black_level_separate_1-label = Black level 1
module-rawprepare-control-raw_black_level_separate_1-full = Black level 1
module-rawprepare-control-raw_black_level_separate_1-promise = Black level 1.
module-rawprepare-control-raw_black_level_separate_2-label = Black level 2
module-rawprepare-control-raw_black_level_separate_2-full = Black level 2
module-rawprepare-control-raw_black_level_separate_2-promise = Black level 2.
module-rawprepare-control-raw_black_level_separate_3-label = Black level 3
module-rawprepare-control-raw_black_level_separate_3-full = Black level 3
module-rawprepare-control-raw_black_level_separate_3-promise = Black level 3.
module-rawprepare-control-raw_white_point-label = White point
module-rawprepare-control-raw_white_point-full = White point
module-rawprepare-control-raw_white_point-promise = White point.
module-rawprepare-control-flat_field-label = Flat field correction
module-rawprepare-control-flat_field-full = Flat field correction
module-rawprepare-control-flat_field-promise = Raw flat field correction to compensate for lens shading.
module-rawprepare-control-flat_field-option-off = Disabled
module-rawprepare-control-flat_field-option-embedded = Embedded GainMap

## Highlight reconstruction (docs/modules/highlight-reconstruction.md): darktable's
## highlight reconstruction, its labels and tooltips in British English.

module-highlights-title = Highlight reconstruction
module-highlights-control-mode-label = Method
module-highlights-control-mode-full = Method
module-highlights-control-mode-promise = Highlight reconstruction method.
module-highlights-control-mode-option-opposed = Inpaint opposed
module-highlights-control-mode-option-lch = Reconstruct in LCh
module-highlights-control-mode-option-clip = Clip highlights
module-highlights-control-mode-option-segments = Segmentation based
module-highlights-control-mode-option-laplacian = Guided laplacians
module-highlights-control-clip-label = Clipping threshold
module-highlights-control-clip-full = Clipping threshold
module-highlights-control-clip-promise = Manually adjust the clipping threshold mostly used against magenta highlights. You might use this for tuning 'Laplacian', 'Inpaint opposed' or 'Segmentation' modes, especially if camera white point is incorrect.
module-highlights-control-combine-label = Combine
module-highlights-control-combine-full = Combine
module-highlights-control-combine-promise = Combine closely related clipped segments by morphological operations. This often leads to improved colour reconstruction for tiny segments before dark background.
module-highlights-control-candidating-label = Candidating
module-highlights-control-candidating-full = Candidating
module-highlights-control-candidating-promise = Select inpainting after segmentation analysis. Increase to favour candidates found in segmentation analysis, decrease for opposed means inpainting.
module-highlights-control-recovery-label = Rebuild
module-highlights-control-recovery-full = Rebuild
module-highlights-control-recovery-promise = Approximate lost data in regions with all photosites clipped, the effect depends on segment size and border gradients. Choose a mode tuned for segment size or the generic mode that tries to find best settings for every segment. Small means areas with a diameter less than 25 pixels, large is best for greater than 100. The flat modes ignore narrow unclipped structures (like power lines) to keep highlights rebuilt and avoid gradients.
module-highlights-control-recovery-option-off = Off
module-highlights-control-recovery-option-generic = Generic
module-highlights-control-recovery-option-flat_generic = Flat generic
module-highlights-control-recovery-option-small_segments = Small segments
module-highlights-control-recovery-option-large_segments = Large segments
module-highlights-control-recovery-option-flat_small_segments = Flat small segments
module-highlights-control-recovery-option-flat_large_segments = Flat large segments
module-highlights-control-strength-label = Strength
module-highlights-control-strength-full = Strength
module-highlights-control-strength-promise = Set strength of rebuilding in regions with all photosites clipped.
module-highlights-control-noise_level-label = Noise level
module-highlights-control-noise_level-full = Noise level
module-highlights-control-noise_level-promise = Add noise to visually blend the reconstructed areas into the rest of the noisy image. Useful at high ISO.
module-highlights-control-iterations-label = Iterations
module-highlights-control-iterations-full = Iterations
module-highlights-control-iterations-promise = Increase if magenta highlights don't get fully corrected. Each new iteration brings a performance penalty.
module-highlights-control-solid_color-label = Inpaint a flat colour
module-highlights-control-solid_color-full = Inpaint a flat colour
module-highlights-control-solid_color-promise = Increase if magenta highlights don't get fully corrected. This may produce non-smooth boundaries between valid and clipped regions.
module-highlights-control-scales-label = Diameter of reconstruction
module-highlights-control-scales-full = Diameter of reconstruction
module-highlights-control-scales-promise = Increase to correct larger clipped areas. Large values bring huge performance penalties.
module-highlights-control-scales-option-px_2 = 2 px
module-highlights-control-scales-option-px_4 = 4 px
module-highlights-control-scales-option-px_8 = 8 px
module-highlights-control-scales-option-px_16 = 16 px
module-highlights-control-scales-option-px_32 = 32 px
module-highlights-control-scales-option-px_64 = 64 px
module-highlights-control-scales-option-px_128 = 128 px (slow)
module-highlights-control-scales-option-px_256 = 256 px (slow)
module-highlights-control-scales-option-px_512 = 512 px (very slow)
module-highlights-control-scales-option-px_1024 = 1024 px (very slow)
module-highlights-control-scales-option-px_2048 = 2048 px (insanely slow)
module-highlights-control-scales-option-px_4096 = 4096 px (insanely slow)

## Demosaic (docs/modules/demosaic.md): darktable's demosaic, its labels and tooltips in
## British English, the method's two tooltips, for Bayer and X-Trans sensors, in one.

module-demosaic-title = Demosaic
module-demosaic-control-demosaicing_method-label = Method
module-demosaic-control-demosaicing_method-full = Demosaicing method
module-demosaic-control-demosaicing_method-promise = Demosaicing method. For a Bayer sensor PPG and RCD are fast, AMaZE and LMMSE are slow; LMMSE is suited best for high ISO images; VNG4 is good in rare cases avoiding maze patterns. For an X-Trans sensor Markesteijn 3-pass and frequency domain chroma are slow. Dual demosaicers increase processing time by blending a VNG variant in a second pass.
module-demosaic-control-demosaicing_method-option-ppg = PPG
module-demosaic-control-demosaicing_method-option-amaze = AMaZE
module-demosaic-control-demosaicing_method-option-vng4 = VNG4
module-demosaic-control-demosaicing_method-option-rcd = RCD
module-demosaic-control-demosaicing_method-option-lmmse = LMMSE
module-demosaic-control-demosaicing_method-option-rcd_dual = RCD (dual)
module-demosaic-control-demosaicing_method-option-amaze_dual = AMaZE (dual)
module-demosaic-control-demosaicing_method-option-passthrough_monochrome = Passthrough (monochrome)
module-demosaic-control-demosaicing_method-option-photosite_color = Photosite colour (debug)
module-demosaic-control-demosaicing_method-option-vng = VNG
module-demosaic-control-demosaicing_method-option-markesteijn = Markesteijn 1-pass
module-demosaic-control-demosaicing_method-option-markesteijn_3 = Markesteijn 3-pass
module-demosaic-control-demosaicing_method-option-fdc = Frequency domain chroma
module-demosaic-control-demosaicing_method-option-markesteijn_3_dual = Markesteijn 3-pass (dual)
module-demosaic-control-demosaicing_method-option-passthrough_monochrome_xtrans = Passthrough (monochrome)
module-demosaic-control-demosaicing_method-option-photosite_color_xtrans = Photosite colour (debug)
module-demosaic-control-demosaicing_method-option-monochrome = Monochrome
module-demosaic-control-dual_thrs-label = Dual threshold
module-demosaic-control-dual_thrs-full = Dual threshold
module-demosaic-control-dual_thrs-promise = Contrast threshold for dual demosaic: 0.0 for high frequency content, 1.0 for flat content.
module-demosaic-control-median_thrs-label = Edge threshold
module-demosaic-control-median_thrs-full = Edge threshold
module-demosaic-control-median_thrs-promise = Threshold for edge-aware median: 0.0 switches it off, 1.0 ignores edges.
module-demosaic-control-lmmse_refine-label = LMMSE refine
module-demosaic-control-lmmse_refine-full = LMMSE refine
module-demosaic-control-lmmse_refine-promise = LMMSE refinement steps. The median steps average the output; refine adds some recalculation of red and blue channels.
module-demosaic-control-lmmse_refine-option-basic = Basic
module-demosaic-control-lmmse_refine-option-median = Median
module-demosaic-control-lmmse_refine-option-median_3x = 3x median
module-demosaic-control-lmmse_refine-option-refine_medians = Refine & medians
module-demosaic-control-lmmse_refine-option-refine_2x_medians = 2x refine + medians
module-demosaic-control-color_smoothing-label = Colour smoothing
module-demosaic-control-color_smoothing-full = Colour smoothing
module-demosaic-control-color_smoothing-promise = How many colour smoothing median steps after demosaicing.
module-demosaic-control-color_smoothing-option-off = Disabled
module-demosaic-control-color_smoothing-option-once = Once
module-demosaic-control-color_smoothing-option-twice = Twice
module-demosaic-control-color_smoothing-option-three = Three times
module-demosaic-control-color_smoothing-option-four = Four times
module-demosaic-control-color_smoothing-option-five = Five times
module-demosaic-control-green_eq-label = Match greens
module-demosaic-control-green_eq-full = Match greens
module-demosaic-control-green_eq-promise = Green channels matching method. Only applies for some old sensors.
module-demosaic-control-green_eq-option-no = Disabled
module-demosaic-control-green_eq-option-local = Local average
module-demosaic-control-green_eq-option-full = Full average
module-demosaic-control-green_eq-option-both = Full and local average
module-demosaic-control-cs_enabled-label = Capture sharpen
module-demosaic-control-cs_enabled-full = Capture sharpen
module-demosaic-control-cs_enabled-promise = Capture sharpen recovers details lost due to in-camera blurring, which can be caused by diffraction, the anti-aliasing filter or other sources of Gaussian-type blur.
module-demosaic-collapsible-capture_sharpen_controls = Capture sharpen controls
module-demosaic-control-cs_iter-label = Iterations
module-demosaic-control-cs_iter-full = Capture sharpen iterations
module-demosaic-control-cs_iter-promise = Set effect strength by iterations.
module-demosaic-control-cs_radius-label = Radius
module-demosaic-control-cs_radius-full = Capture sharpen radius
module-demosaic-control-cs_radius-promise = Capture sharpen radius should reflect the overall Gaussian-type blur of the camera sensor, possibly the anti-aliasing filter and the lens. Increasing this too far will soon lead to artefacts like halos and ringing, especially when used with a large iterations setting. A radius set to zero will be recalculated automatically the next run; use for presets.
module-demosaic-control-cs_thrs-label = Contrast sensitivity
module-demosaic-control-cs_thrs-full = Capture sharpen contrast sensitivity
module-demosaic-control-cs_thrs-promise = Restrict capture sharpening to areas with high local contrast: increase to exclude flat areas in very dark or noisy images, decrease for well exposed and low noise images. A threshold set to zero will be reset to defaults the next run; use for presets.
module-demosaic-control-cs_boost-label = Corner boost
module-demosaic-control-cs_boost-full = Capture sharpen corner boost
module-demosaic-control-cs_boost-promise = Further increase sharpen radius at image corners; the sharp centre of the image will not be affected.
module-demosaic-control-cs_center-label = Sharp centre
module-demosaic-control-cs_center-full = Capture sharpen sharp centre
module-demosaic-control-cs_center-promise = Adjust to the sharp image centre.

## AgX (docs/modules/agx.md): darktable's AgX, its labels and tooltips in British English.
## A full name says which group a short label is in, as darktable's shortcuts do.

module-agx-title = AgX
module-agx-tab-settings = Settings
module-agx-tab-primaries = Primaries
module-agx-group-input_exposure_range = Input exposure range
module-agx-group-basic_curve_parameters = Basic curve parameters
module-agx-group-before_tone_mapping = Before tone mapping
module-agx-group-after_tone_mapping = After tone mapping
module-agx-collapsible-show_curve = Show curve
module-agx-collapsible-advanced_curve_parameters = Advanced curve parameters
module-agx-collapsible-look = Look
module-agx-control-range_white_relative_ev-label = White relative exposure
module-agx-control-range_white_relative_ev-full = White relative exposure
module-agx-control-range_white_relative_ev-promise = Relative exposure above mid-grey (white point).
module-agx-control-range_black_relative_ev-label = Black relative exposure
module-agx-control-range_black_relative_ev-full = Black relative exposure
module-agx-control-range_black_relative_ev-promise = Relative exposure below mid-grey (black point).
module-agx-control-dynamic_range_scaling-label = Dynamic range scaling
module-agx-control-dynamic_range_scaling-full = Dynamic range scaling
module-agx-control-dynamic_range_scaling-promise = Symmetrically increase or decrease the computed dynamic range. Useful to give a safety margin to extreme luminances.
module-agx-action-auto_tune_levels-label = Auto tune levels
module-agx-action-auto_tune_levels-full = Auto tune levels
module-agx-action-auto_tune_levels-promise = Set black and white relative exposure using the selected area.
module-agx-action-read_exposure-label = Read exposure
module-agx-action-read_exposure-full = Read exposure
module-agx-action-read_exposure-promise = Read exposure from the metadata and the Exposure module.
module-agx-control-curve_pivot_x-label = Pivot relative exposure
module-agx-control-curve_pivot_x-full = Pivot relative exposure
module-agx-control-curve_pivot_x-promise = Set the pivot's input exposure in EV relative to mid-grey.
module-agx-control-curve_pivot_y_linear_output-label = Pivot target output
module-agx-control-curve_pivot_y_linear_output-full = Pivot target output
module-agx-control-curve_pivot_y_linear_output-promise = Darken or brighten the pivot (linear output power).
module-agx-control-curve_contrast_around_pivot-label = Contrast
module-agx-control-curve_contrast_around_pivot-full = Curve contrast
module-agx-control-curve_contrast_around_pivot-promise = Slope of the linear section around the pivot.
module-agx-control-curve_shoulder_power-label = Shoulder power
module-agx-control-curve_shoulder_power-full = Shoulder power
module-agx-control-curve_shoulder_power-promise = Contrast in highlights. Higher values keep the slope nearly constant for longer, at the cost of a more sudden drop near white.
module-agx-control-curve_toe_power-label = Toe power
module-agx-control-curve_toe_power-full = Toe power
module-agx-control-curve_toe_power-promise = Contrast in shadows. Higher values keep the slope nearly constant for longer, at the cost of a more sudden drop near black.
module-agx-control-curve_linear_ratio_above_pivot-label = Shoulder start
module-agx-control-curve_linear_ratio_above_pivot-full = Shoulder start
module-agx-control-curve_linear_ratio_above_pivot-promise = Length to keep the curve linear above the pivot. May clip highlights.
module-agx-control-curve_target_display_white_ratio-label = Target white
module-agx-control-curve_target_display_white_ratio-full = Target white
module-agx-control-curve_target_display_white_ratio-promise = Maximum linear output power.
module-agx-control-curve_linear_ratio_below_pivot-label = Toe start
module-agx-control-curve_linear_ratio_below_pivot-full = Toe start
module-agx-control-curve_linear_ratio_below_pivot-promise = Length to keep the curve linear below the pivot. May crush shadows.
module-agx-control-curve_target_display_black_ratio-label = Target black
module-agx-control-curve_target_display_black_ratio-full = Target black
module-agx-control-curve_target_display_black_ratio-promise = Raise for a faded look.
module-agx-control-auto_gamma-label = Keep the pivot on the diagonal
module-agx-control-auto_gamma-full = Keep the pivot on the diagonal
module-agx-control-auto_gamma-promise = Adjusts the gamma automatically, trying to make sure the curve always remains S-shaped (given that contrast is high enough), so the toe and shoulder controls remain effective.
module-agx-control-curve_gamma-label = Curve y gamma
module-agx-control-curve_gamma-full = Curve y gamma
module-agx-control-curve_gamma-promise = Shifts the representation (but not the output power) of the pivot along the y axis of the curve. Immediate contrast around the pivot is not affected, but shadows and highlights are; you may have to counteract it with the contrast slider or with the toe and shoulder controls.
module-agx-control-look_slope-label = Slope
module-agx-control-look_slope-full = Look slope
module-agx-control-look_slope-promise = Decrease or increase contrast and brightness.
module-agx-control-look_lift-label = Lift
module-agx-control-look_lift-full = Look lift
module-agx-control-look_lift-promise = Deepen or lift shadows.
module-agx-control-look_brightness-label = Brightness
module-agx-control-look_brightness-full = Look brightness
module-agx-control-look_brightness-promise = Increase or decrease brightness.
module-agx-control-look_saturation-label = Saturation
module-agx-control-look_saturation-full = Look saturation
module-agx-control-look_saturation-promise = Decrease or increase saturation.
module-agx-control-look_original_hue_mix_ratio-label = Preserve hue
module-agx-control-look_original_hue_mix_ratio-full = Preserve hue
module-agx-control-look_original_hue_mix_ratio-promise = Increase to bring hues closer to the original.
module-agx-control-base_primaries-label = Base primaries
module-agx-control-base_primaries-full = Base primaries
module-agx-control-base_primaries-promise = Colour space primaries to use as the base for the adjustments below. Export profile uses the profile set in Output colour profile.
module-agx-control-base_primaries-option-export_profile = Export profile
module-agx-control-base_primaries-option-working_profile = Working profile
module-agx-control-base_primaries-option-rec2020 = Rec2020
module-agx-control-base_primaries-option-display_p3 = Display P3
module-agx-control-base_primaries-option-adobe_rgb = Adobe RGB (compatible)
module-agx-control-base_primaries-option-srgb = sRGB
module-agx-control-disable_primaries_adjustments-label = Disable adjustments
module-agx-control-disable_primaries_adjustments-full = Disable primaries adjustments
module-agx-control-disable_primaries_adjustments-promise = Disable purity adjustments and rotations, only applying the curve. Note that those adjustments are at the heart of AgX: without them the results are almost always going to be worse, especially with bright, saturated lights (e.g. LEDs). Mainly intended to be used for experimenting.
module-agx-action-reset_primaries-label = Reset primaries
module-agx-action-reset_primaries-full = Reset primaries
module-agx-action-reset_primaries-promise = Reset primaries to a predefined configuration.
module-agx-control-red_inset-label = Red attenuation
module-agx-control-red_inset-full = Red attenuation
module-agx-control-red_inset-promise = Increase to desaturate reds in highlights faster.
module-agx-control-red_rotation-label = Red rotation
module-agx-control-red_rotation-full = Red rotation
module-agx-control-red_rotation-promise = Shift the red primary towards yellow (+) or magenta (−).
module-agx-control-green_inset-label = Green attenuation
module-agx-control-green_inset-full = Green attenuation
module-agx-control-green_inset-promise = Increase to desaturate greens in highlights faster.
module-agx-control-green_rotation-label = Green rotation
module-agx-control-green_rotation-full = Green rotation
module-agx-control-green_rotation-promise = Shift the green primary towards cyan (+) or yellow (−).
module-agx-control-blue_inset-label = Blue attenuation
module-agx-control-blue_inset-full = Blue attenuation
module-agx-control-blue_inset-promise = Increase to desaturate blues in highlights faster.
module-agx-control-blue_rotation-label = Blue rotation
module-agx-control-blue_rotation-full = Blue rotation
module-agx-control-blue_rotation-promise = Shift the blue primary towards magenta (+) or cyan (−).
module-agx-control-completely_reverse_primaries-label = Reverse all
module-agx-control-completely_reverse_primaries-full = Reverse all
module-agx-control-completely_reverse_primaries-promise = Completely restore purity, undo all rotations, and hide the controls below. Uncheck to restore the previous state.
module-agx-action-set_from_above-label = Set from above
module-agx-action-set_from_above-full = Set from above
module-agx-action-set_from_above-promise = Set parameters to completely reverse primaries modifications, but allow subsequent editing.
module-agx-control-master_outset_ratio-label = Master purity boost
module-agx-control-master_outset_ratio-full = Master purity boost
module-agx-control-master_outset_ratio-promise = Overall purity boost.
module-agx-control-master_unrotation_ratio-label = Master rotation reversal
module-agx-control-master_unrotation_ratio-full = Master rotation reversal
module-agx-control-master_unrotation_ratio-promise = Overall unrotation ratio.
module-agx-control-red_outset-label = Red purity boost
module-agx-control-red_outset-full = Red purity boost
module-agx-control-red_outset-promise = Restore the purity of red, mostly in mid-tones and shadows.
module-agx-control-red_unrotation-label = Red reverse rotation
module-agx-control-red_unrotation-full = Red reverse rotation
module-agx-control-red_unrotation-promise = Reverse the colour shift in reds.
module-agx-control-green_outset-label = Green purity boost
module-agx-control-green_outset-full = Green purity boost
module-agx-control-green_outset-promise = Restore the purity of green, mostly in mid-tones and shadows.
module-agx-control-green_unrotation-label = Green reverse rotation
module-agx-control-green_unrotation-full = Green reverse rotation
module-agx-control-green_unrotation-promise = Reverse the colour shift in greens.
module-agx-control-blue_outset-label = Blue purity boost
module-agx-control-blue_outset-full = Blue purity boost
module-agx-control-blue_outset-promise = Restore the purity of blue, mostly in mid-tones and shadows.
module-agx-control-blue_unrotation-label = Blue reverse rotation
module-agx-control-blue_unrotation-full = Blue reverse rotation
module-agx-control-blue_unrotation-promise = Reverse the colour shift in blues.
## Output colour profile (docs/modules/output-colour-profile.md): darktable's output color
## profile, its label and its profiles' names in British English. darktable's tooltip
## names the folders it loads ICC files from, which Apertrust does not read yet.

module-colorout-title = Output colour profile
module-colorout-control-type-label = Export profile
module-colorout-control-type-full = Export profile
module-colorout-control-type-promise = The colour profile the image is exported in.
module-colorout-control-type-option-srgb = sRGB (web-safe)
module-colorout-control-type-option-adobe_rgb = Adobe RGB (compatible)
module-colorout-control-type-option-lin_rec709 = Linear Rec709 RGB
module-colorout-control-type-option-rec709 = Rec709 RGB
module-colorout-control-type-option-lin_rec2020 = Linear Rec2020 RGB
module-colorout-control-type-option-pq_rec2020 = PQ Rec2020 RGB
module-colorout-control-type-option-hlg_rec2020 = HLG Rec2020 RGB
module-colorout-control-type-option-pq_p3 = PQ P3 RGB
module-colorout-control-type-option-hlg_p3 = HLG P3 RGB
module-colorout-control-type-option-display_p3 = Display P3 RGB
module-colorout-control-type-option-prophoto_rgb = Linear ProPhoto RGB
module-colorout-control-type-option-brg = BRG (for testing)

## White balance (docs/modules/white-balance.md): darktable's white balance, its labels
## and tooltips in British English.

module-temperature-title = White balance
module-temperature-preset-settings-label = Settings
module-temperature-preset-settings-full = White balance settings
module-temperature-preset-settings-promise = Chooses the white balance setting: the coefficients and where they come from.
module-temperature-preset-settings-option-as_shot = As shot
module-temperature-preset-settings-option-spot = From image area
module-temperature-preset-settings-option-user = User modified
module-temperature-preset-settings-option-d65 = Camera reference
module-temperature-preset-settings-option-d65_late = As shot to reference
module-temperature-control-preset-label = Settings
module-temperature-control-preset-full = White balance settings
module-temperature-control-preset-promise = Where the coefficients come from. As shot to reference sets white balance to as shot and later corrects it to the camera's reference point, in most cases D65.
module-temperature-control-preset-option-as_shot = As shot
module-temperature-control-preset-option-spot = From image area
module-temperature-control-preset-option-user = User modified
module-temperature-control-preset-option-d65 = Camera reference
module-temperature-control-preset-option-d65_late = As shot to reference
module-temperature-group-scene_illuminant_temp = Scene illuminant temp
module-temperature-control-temperature-label = Temperature
module-temperature-control-temperature-full = Scene illuminant temperature
module-temperature-control-temperature-promise = The colour temperature, in kelvin.
module-temperature-control-tint-label = Tint
module-temperature-control-tint-full = Scene illuminant tint
module-temperature-control-tint-promise = The colour tint of the image, from magenta (below 1) to green (above 1).
module-temperature-collapsible-channel_coefficients = Channel coefficients
module-temperature-control-red-label = Red
module-temperature-control-red-full = Red coefficient
module-temperature-control-red-promise = What the raw's red photosites are multiplied by.
module-temperature-control-green-label = Green
module-temperature-control-green-full = Green coefficient
module-temperature-control-green-promise = What the raw's green photosites are multiplied by.
module-temperature-control-blue-label = Blue
module-temperature-control-blue-full = Blue coefficient
module-temperature-control-blue-promise = What the raw's blue photosites are multiplied by.
module-temperature-control-various-label = Emerald
module-temperature-control-various-full = Emerald coefficient
module-temperature-control-various-promise = What a four-colour sensor's emerald photosites are multiplied by.

## Orientation (docs/modules/orientation.md): darktable's orientation, its label, its
## buttons and its orientations by darktable's names in British English. darktable shows
## the stored orientation under its Transform label only as the photo itself.

module-flip-title = Orientation
module-flip-control-orientation-label = Transform
module-flip-control-orientation-full = Transform
module-flip-control-orientation-promise = Flip or rotate the image by steps of 90°.
module-flip-control-orientation-option-auto = Autodetect
module-flip-control-orientation-option-none = No rotation
module-flip-control-orientation-option-flip_vertically = Flip vertically
module-flip-control-orientation-option-flip_horizontally = Flip horizontally
module-flip-control-orientation-option-rotate_180 = Rotate 180°
module-flip-control-orientation-option-transpose = Transpose
module-flip-control-orientation-option-rotate_cw_90 = Rotate −90°
module-flip-control-orientation-option-rotate_ccw_90 = Rotate 90°
module-flip-control-orientation-option-transverse = Transverse
module-flip-action-rotate_ccw-label = Rotate 90° anticlockwise
module-flip-action-rotate_ccw-full = Rotate 90° anticlockwise
module-flip-action-rotate_ccw-promise = Rotate the image 90° anticlockwise.
module-flip-action-rotate_cw-label = Rotate 90° clockwise
module-flip-action-rotate_cw-full = Rotate 90° clockwise
module-flip-action-rotate_cw-promise = Rotate the image 90° clockwise.
module-flip-action-flip_horizontally-label = Flip horizontally
module-flip-action-flip_horizontally-full = Flip horizontally
module-flip-action-flip_horizontally-promise = Mirror the image left to right.
module-flip-action-flip_vertically-label = Flip vertically
module-flip-action-flip_vertically-full = Flip vertically
module-flip-action-flip_vertically-promise = Mirror the image top to bottom.

## Exposure (docs/modules/exposure.md): darktable's exposure, its labels and tooltips in
## British English. The promises of Exposure, Compensate camera exposure and Black level
## correction are its invariants table's.

module-exposure-title = Exposure
module-exposure-auto = Sets Exposure so the photo's tones sit around middle grey. A kept Exposure stays as it is.
module-exposure-control-mode-label = Mode
module-exposure-control-mode-full = Mode
module-exposure-control-mode-promise = Manual sets the exposure by hand. Automatic sets it from each photo's raw histogram, to even out the exposure of a series.
module-exposure-control-mode-option-manual = Manual
module-exposure-control-mode-option-automatic = Automatic
module-exposure-control-camera_bias-label = Compensate camera exposure
module-exposure-control-camera_bias-full = Compensate camera exposure
module-exposure-control-camera_bias-promise = Undoes the exposure compensation set on the camera.
module-exposure-control-compensate_hilite_pres-label = Highlight preservation mode
module-exposure-control-compensate_hilite_pres-full = Highlight preservation mode
module-exposure-control-compensate_hilite_pres-promise = Removes the camera's hidden exposure bias in HDR, highlight preservation, dynamic range or HLG tone mode. On an image with a bias, it needs tone mapping, such as sigmoid, to avoid blown-out highlights.
module-exposure-control-exposure-label = Exposure
module-exposure-control-exposure-full = Exposure
module-exposure-control-exposure-promise = Adjusts the exposure correction.
module-exposure-control-deflicker_percentile-label = Percentile
module-exposure-control-deflicker_percentile-full = Percentile
module-exposure-control-deflicker_percentile-promise = Where in the histogram to meter for deflickering: 50% is the median.
module-exposure-control-deflicker_target_level-label = Target level
module-exposure-control-deflicker_target_level-full = Target level
module-exposure-control-deflicker_target_level-promise = Where to place the exposure of processed photos, in EV below overexposure.
module-exposure-control-black_level-label = Black level correction
module-exposure-control-black_level-full = Black level correction
module-exposure-control-black_level-promise = Sets which level is black. Keeps white. 0 = untouched.
module-exposure-collapsible-area_exposure_mapping = Area exposure mapping
module-exposure-setting-area_mode-label = Area mode
module-exposure-setting-area_mode-full = Area mode
module-exposure-setting-area_mode-promise = Correction adjusts the exposure so that the input's lightness is mapped to the target. Measure only shows how an input colour is mapped by the exposure, and can be used to define a target.
module-exposure-setting-area_mode-option-correction = Correction
module-exposure-setting-area_mode-option-measure = Measure
module-exposure-group-target = Target
module-exposure-setting-lightness-label = Lightness
module-exposure-setting-lightness-full = Target lightness
module-exposure-setting-lightness-promise = The desired target exposure after mapping.

## Colour calibration (docs/modules/colour-calibration.md): darktable's color calibration,
## every tab, the CAT tab first. Its words are darktable's, the adaptation's and the area
## colour mapping's promises its tooltips; darktable gives its other CAT controls none, so
## their promises say what each describes.

module-channelmixerrgb-title = Colour calibration
module-channelmixerrgb-tab-cat = CAT
module-channelmixerrgb-control-adaptation-label = Adaptation
module-channelmixerrgb-control-adaptation-full = Adaptation
module-channelmixerrgb-control-adaptation-promise = How the illuminant is adapted, and the colour space the module works in. Linear Bradford (1985) is consistent with the ICC v4 toolchain. CAT16 (2016) is more robust and accurate. Non-linear Bradford (1985) is the original Bradford: it can give better results than the linear one, but is unreliable. XYZ is a simple scaling in XYZ space, not recommended in general. None disables any adaptation and uses the pipeline's working RGB.
module-channelmixerrgb-control-adaptation-option-linear_bradford = Linear Bradford (ICC v4)
module-channelmixerrgb-control-adaptation-option-cat16 = CAT16 (CIECAM16)
module-channelmixerrgb-control-adaptation-option-non_linear_bradford = Non-linear Bradford
module-channelmixerrgb-control-adaptation-option-xyz = XYZ
module-channelmixerrgb-control-adaptation-option-none = None (bypass)
module-channelmixerrgb-control-illuminant-label = Illuminant
module-channelmixerrgb-control-illuminant-full = Illuminant
module-channelmixerrgb-control-illuminant-promise = The light the scene was lit by, which the adaptation turns into white: a standard light, one at a temperature, the camera's own or a custom colour.
module-channelmixerrgb-control-illuminant-option-pipeline = Same as pipeline (D50)
module-channelmixerrgb-control-illuminant-option-a = A (incandescent)
module-channelmixerrgb-control-illuminant-option-d = D (daylight)
module-channelmixerrgb-control-illuminant-option-e = E (equi-energy)
module-channelmixerrgb-control-illuminant-option-f = F (fluorescent)
module-channelmixerrgb-control-illuminant-option-led = LED (LED light)
module-channelmixerrgb-control-illuminant-option-planckian = Planckian (black body)
module-channelmixerrgb-control-illuminant-option-custom = Custom
module-channelmixerrgb-control-illuminant-option-camera = As shot in camera
module-channelmixerrgb-control-illum_fluo-label = F source
module-channelmixerrgb-control-illum_fluo-full = Fluorescent source
module-channelmixerrgb-control-illum_fluo-promise = Which CIE fluorescent light lit the scene.
module-channelmixerrgb-control-illum_fluo-option-f1 = F1 (Daylight 6430 K) – medium CRI
module-channelmixerrgb-control-illum_fluo-option-f2 = F2 (Cool White 4230 K) – medium CRI
module-channelmixerrgb-control-illum_fluo-option-f3 = F3 (White 3450 K) – medium CRI
module-channelmixerrgb-control-illum_fluo-option-f4 = F4 (Warm White 2940 K) – medium CRI
module-channelmixerrgb-control-illum_fluo-option-f5 = F5 (Daylight 6350 K) – medium CRI
module-channelmixerrgb-control-illum_fluo-option-f6 = F6 (Lite White 4150 K) – medium CRI
module-channelmixerrgb-control-illum_fluo-option-f7 = F7 (D65 simulator 6500 K) – high CRI
module-channelmixerrgb-control-illum_fluo-option-f8 = F8 (D50 simulator 5000 K) – high CRI
module-channelmixerrgb-control-illum_fluo-option-f9 = F9 (Cool White Deluxe 4150 K) – high CRI
module-channelmixerrgb-control-illum_fluo-option-f10 = F10 (Tuned RGB 5000 K) – low CRI
module-channelmixerrgb-control-illum_fluo-option-f11 = F11 (Tuned RGB 4000 K) – low CRI
module-channelmixerrgb-control-illum_fluo-option-f12 = F12 (Tuned RGB 3000 K) – low CRI
module-channelmixerrgb-control-illum_led-label = LED source
module-channelmixerrgb-control-illum_led-full = LED source
module-channelmixerrgb-control-illum_led-promise = Which CIE LED light lit the scene.
module-channelmixerrgb-control-illum_led-option-b1 = B1 (Blue 2733 K)
module-channelmixerrgb-control-illum_led-option-b2 = B2 (Blue 2998 K)
module-channelmixerrgb-control-illum_led-option-b3 = B3 (Blue 4103 K)
module-channelmixerrgb-control-illum_led-option-b4 = B4 (Blue 5109 K)
module-channelmixerrgb-control-illum_led-option-b5 = B5 (Blue 6598 K)
module-channelmixerrgb-control-illum_led-option-bh1 = BH1 (Blue-Red hybrid 2851 K)
module-channelmixerrgb-control-illum_led-option-rgb1 = RGB1 (RGB 2840 K)
module-channelmixerrgb-control-illum_led-option-v1 = V1 (Violet 2724 K)
module-channelmixerrgb-control-illum_led-option-v2 = V2 (Violet 4070 K)
module-channelmixerrgb-control-temperature-label = Temperature
module-channelmixerrgb-control-temperature-full = Illuminant temperature
module-channelmixerrgb-control-temperature-promise = The temperature of the light, on the daylight locus or the black body's.
module-channelmixerrgb-control-x-label = x
module-channelmixerrgb-control-x-full = Illuminant x
module-channelmixerrgb-control-x-promise = The light's CIE 1931 x, as the pipeline sees it. Hue and chroma show it.
module-channelmixerrgb-control-y-label = y
module-channelmixerrgb-control-y-full = Illuminant y
module-channelmixerrgb-control-y-promise = The light's CIE 1931 y, as the pipeline sees it. Hue and chroma show it.
module-channelmixerrgb-control-hue-label = Hue
module-channelmixerrgb-control-hue-full = Illuminant hue
module-channelmixerrgb-control-hue-promise = The colour of the light, around the pipeline's white.
module-channelmixerrgb-control-chroma-label = Chroma
module-channelmixerrgb-control-chroma-full = Illuminant chroma
module-channelmixerrgb-control-chroma-promise = How far the light is from the pipeline's white. 0 = white.
module-channelmixerrgb-control-gamut-label = Gamut compression
module-channelmixerrgb-control-gamut-full = Gamut compression
module-channelmixerrgb-control-gamut-promise = Brings colours outside the pipeline's gamut back inside it, the more the higher. 0 = none.
module-channelmixerrgb-control-clip-label = Clip negative RGB from gamut
module-channelmixerrgb-control-clip-full = Clip negative RGB from gamut
module-channelmixerrgb-control-clip-promise = Clips the negative RGB values the adaptation leaves outside the gamut.

# The area colour mapping's settings, darktable's own and never in an edit, drawn disabled
# until the picker comes.
module-channelmixerrgb-collapsible-area_colour_mapping = Area colour mapping
module-channelmixerrgb-setting-area_mode-label = Area mode
module-channelmixerrgb-setting-area_mode-full = Area mode
module-channelmixerrgb-setting-area_mode-promise = Correction adjusts the illuminant so the input colour is mapped to the target. Measure only shows how an input colour is mapped by the CAT, and can be used to sample a target.
module-channelmixerrgb-setting-area_mode-option-correction = Correction
module-channelmixerrgb-setting-area_mode-option-measure = Measure
module-channelmixerrgb-setting-channel_mixing-label = Take channel mixing into account
module-channelmixerrgb-setting-channel_mixing-full = Take channel mixing into account
module-channelmixerrgb-setting-channel_mixing-promise = Computes the target taking the channel mixing into account. Off, only the CAT is considered.
module-channelmixerrgb-group-target = Target
module-channelmixerrgb-setting-target_lightness-label = Lightness
module-channelmixerrgb-setting-target_lightness-full = Target lightness
module-channelmixerrgb-setting-target_lightness-promise = The lightness of the desired target colour after mapping.
module-channelmixerrgb-setting-target_hue-label = Hue
module-channelmixerrgb-setting-target_hue-full = Target hue
module-channelmixerrgb-setting-target_hue-promise = The hue of the desired target colour after mapping.
module-channelmixerrgb-setting-target_chroma-label = Chroma
module-channelmixerrgb-setting-target_chroma-full = Target chroma
module-channelmixerrgb-setting-target_chroma-promise = The chroma of the desired target colour after mapping.

module-channelmixerrgb-tab-red = R
module-channelmixerrgb-tab-green = G
module-channelmixerrgb-tab-blue = B
module-channelmixerrgb-tab-colourfulness = Colourfulness
module-channelmixerrgb-tab-brightness = Brightness
module-channelmixerrgb-tab-grey = Grey
module-channelmixerrgb-control-red_r-label = Input R
module-channelmixerrgb-control-red_r-full = Red, input R
module-channelmixerrgb-control-red_r-promise = How much of the input's R the output's R takes. 1 = untouched.
module-channelmixerrgb-control-red_g-label = Input G
module-channelmixerrgb-control-red_g-full = Red, input G
module-channelmixerrgb-control-red_g-promise = How much of the input's G the output's R takes. 0 = untouched.
module-channelmixerrgb-control-red_b-label = Input B
module-channelmixerrgb-control-red_b-full = Red, input B
module-channelmixerrgb-control-red_b-promise = How much of the input's B the output's R takes. 0 = untouched.
module-channelmixerrgb-control-red_normalise-label = Normalise channels
module-channelmixerrgb-control-red_normalise-full = Red, normalise channels
module-channelmixerrgb-control-red_normalise-promise = Scales the output's R so its three inputs add up to 1.
module-channelmixerrgb-control-green_r-label = Input R
module-channelmixerrgb-control-green_r-full = Green, input R
module-channelmixerrgb-control-green_r-promise = How much of the input's R the output's G takes. 0 = untouched.
module-channelmixerrgb-control-green_g-label = Input G
module-channelmixerrgb-control-green_g-full = Green, input G
module-channelmixerrgb-control-green_g-promise = How much of the input's G the output's G takes. 1 = untouched.
module-channelmixerrgb-control-green_b-label = Input B
module-channelmixerrgb-control-green_b-full = Green, input B
module-channelmixerrgb-control-green_b-promise = How much of the input's B the output's G takes. 0 = untouched.
module-channelmixerrgb-control-green_normalise-label = Normalise channels
module-channelmixerrgb-control-green_normalise-full = Green, normalise channels
module-channelmixerrgb-control-green_normalise-promise = Scales the output's G so its three inputs add up to 1.
module-channelmixerrgb-control-blue_r-label = Input R
module-channelmixerrgb-control-blue_r-full = Blue, input R
module-channelmixerrgb-control-blue_r-promise = How much of the input's R the output's B takes. 0 = untouched.
module-channelmixerrgb-control-blue_g-label = Input G
module-channelmixerrgb-control-blue_g-full = Blue, input G
module-channelmixerrgb-control-blue_g-promise = How much of the input's G the output's B takes. 0 = untouched.
module-channelmixerrgb-control-blue_b-label = Input B
module-channelmixerrgb-control-blue_b-full = Blue, input B
module-channelmixerrgb-control-blue_b-promise = How much of the input's B the output's B takes. 1 = untouched.
module-channelmixerrgb-control-blue_normalise-label = Normalise channels
module-channelmixerrgb-control-blue_normalise-full = Blue, normalise channels
module-channelmixerrgb-control-blue_normalise-promise = Scales the output's B so its three inputs add up to 1.
module-channelmixerrgb-control-colourfulness_r-label = Input R
module-channelmixerrgb-control-colourfulness_r-full = Colourfulness, input R
module-channelmixerrgb-control-colourfulness_r-promise = Makes colours more or less colourful by their R, keeping their brightness. 0 = untouched.
module-channelmixerrgb-control-colourfulness_g-label = Input G
module-channelmixerrgb-control-colourfulness_g-full = Colourfulness, input G
module-channelmixerrgb-control-colourfulness_g-promise = Makes colours more or less colourful by their G, keeping their brightness. 0 = untouched.
module-channelmixerrgb-control-colourfulness_b-label = Input B
module-channelmixerrgb-control-colourfulness_b-full = Colourfulness, input B
module-channelmixerrgb-control-colourfulness_b-promise = Makes colours more or less colourful by their B, keeping their brightness. 0 = untouched.
module-channelmixerrgb-control-colourfulness_normalise-label = Normalise channels
module-channelmixerrgb-control-colourfulness_normalise-full = Colourfulness, normalise channels
module-channelmixerrgb-control-colourfulness_normalise-promise = Takes the three weights' mean from each, so the average colour keeps its colourfulness.
module-channelmixerrgb-control-brightness_r-label = Input R
module-channelmixerrgb-control-brightness_r-full = Brightness, input R
module-channelmixerrgb-control-brightness_r-promise = Brightens or darkens each pixel by its R. 0 = untouched.
module-channelmixerrgb-control-brightness_g-label = Input G
module-channelmixerrgb-control-brightness_g-full = Brightness, input G
module-channelmixerrgb-control-brightness_g-promise = Brightens or darkens each pixel by its G. 0 = untouched.
module-channelmixerrgb-control-brightness_b-label = Input B
module-channelmixerrgb-control-brightness_b-full = Brightness, input B
module-channelmixerrgb-control-brightness_b-promise = Brightens or darkens each pixel by its B. 0 = untouched.
module-channelmixerrgb-control-brightness_normalise-label = Normalise channels
module-channelmixerrgb-control-brightness_normalise-full = Brightness, normalise channels
module-channelmixerrgb-control-brightness_normalise-promise = Takes the three weights' mean from each, so the average colour keeps its brightness.
module-channelmixerrgb-control-grey_r-label = Input R
module-channelmixerrgb-control-grey_r-full = Grey, input R
module-channelmixerrgb-control-grey_r-promise = How much of the input's R the grey takes. Any weight off 0 makes the photo grey. 0 = untouched.
module-channelmixerrgb-control-grey_g-label = Input G
module-channelmixerrgb-control-grey_g-full = Grey, input G
module-channelmixerrgb-control-grey_g-promise = How much of the input's G the grey takes. Any weight off 0 makes the photo grey. 0 = untouched.
module-channelmixerrgb-control-grey_b-label = Input B
module-channelmixerrgb-control-grey_b-full = Grey, input B
module-channelmixerrgb-control-grey_b-promise = How much of the input's B the grey takes. Any weight off 0 makes the photo grey. 0 = untouched.
module-channelmixerrgb-control-grey_normalise-label = Normalise channels
module-channelmixerrgb-control-grey_normalise-full = Grey, normalise channels
module-channelmixerrgb-control-grey_normalise-promise = Scales the grey's three weights so they add up to 1, keeping the photo's brightness.
# The colour checker's settings, darktable's own and never in an edit, drawn disabled until
# the tool comes.
module-channelmixerrgb-collapsible-colour_checker = Calibrate with a colour checker
module-channelmixerrgb-setting-chart-label = Chart
module-channelmixerrgb-setting-chart-full = Colour checker chart
module-channelmixerrgb-setting-chart-promise = The vendor and the type of your chart.
module-channelmixerrgb-setting-chart-option-xrite_24_2000 = Xrite ColorChecker 24 pre-2014
module-channelmixerrgb-setting-chart-option-xrite_24_2014 = Xrite/Calibrite ColorChecker 24 post-2014
module-channelmixerrgb-setting-chart-option-spyder_24 = Datacolor SpyderCheckr 24 pre-2018
module-channelmixerrgb-setting-chart-option-spyder_24_v2 = Datacolor SpyderCheckr 24 post-2018
module-channelmixerrgb-setting-chart-option-spyder_48 = Datacolor SpyderCheckr 48 pre-2018
module-channelmixerrgb-setting-chart-option-spyder_48_v2 = Datacolor SpyderCheckr 48 post-2018
module-channelmixerrgb-setting-chart-option-spyder_photo = Datacolor SpyderCheckr Photo
module-channelmixerrgb-setting-optimise_for-label = Optimise for
module-channelmixerrgb-setting-optimise_for-full = Colours the colour checker favours
module-channelmixerrgb-setting-optimise_for-promise = The colours the fit gets closest. Neutral colours give the lowest average ΔE, saturated colours the lowest maximum.
module-channelmixerrgb-setting-optimise_for-option-none = None
module-channelmixerrgb-setting-optimise_for-option-neutral = Neutral colours
module-channelmixerrgb-setting-optimise_for-option-saturated = Saturated colours
module-channelmixerrgb-setting-optimise_for-option-skin = Skin and soil colours
module-channelmixerrgb-setting-optimise_for-option-foliage = Foliage colours
module-channelmixerrgb-setting-optimise_for-option-sky = Sky and water colours
module-channelmixerrgb-setting-optimise_for-option-average_delta_e = Average ΔE
module-channelmixerrgb-setting-optimise_for-option-maximum_delta_e = Maximum ΔE
module-channelmixerrgb-setting-patch_scale-label = Patch scale
module-channelmixerrgb-setting-patch_scale-full = Colour checker patch scale
module-channelmixerrgb-setting-patch_scale-promise = How much of each patch is read, from its centre: less where the chart's frame is loose or casts shadows.

## Tone equalizer (docs/modules/tone-equalizer.md): darktable's tone equalizer, its labels
## and tooltips in British English. A band's full name adds darktable's word for its tones;
## darktable gives a band no tooltip, so its promise says which tones it corrects.

module-toneequal-title = Tone equalizer
module-toneequal-tab-simple = Simple
module-toneequal-tab-advanced = Advanced
module-toneequal-tab-masking = Masking
module-toneequal-control-noise-label = −8 EV
module-toneequal-control-noise-full = −8 EV, blacks
module-toneequal-control-noise-promise = Brightens or darkens the blacks, the tones at −8 EV in the mask.
module-toneequal-control-ultra_deep_blacks-label = −7 EV
module-toneequal-control-ultra_deep_blacks-full = −7 EV, deep shadows
module-toneequal-control-ultra_deep_blacks-promise = Brightens or darkens the deep shadows, the tones at −7 EV in the mask.
module-toneequal-control-deep_blacks-label = −6 EV
module-toneequal-control-deep_blacks-full = −6 EV, shadows
module-toneequal-control-deep_blacks-promise = Brightens or darkens the shadows, the tones at −6 EV in the mask.
module-toneequal-control-blacks-label = −5 EV
module-toneequal-control-blacks-full = −5 EV, light shadows
module-toneequal-control-blacks-promise = Brightens or darkens the light shadows, the tones at −5 EV in the mask.
module-toneequal-control-shadows-label = −4 EV
module-toneequal-control-shadows-full = −4 EV, mid-tones
module-toneequal-control-shadows-promise = Brightens or darkens the mid-tones, the tones at −4 EV in the mask.
module-toneequal-control-midtones-label = −3 EV
module-toneequal-control-midtones-full = −3 EV, dark highlights
module-toneequal-control-midtones-promise = Brightens or darkens the dark highlights, the tones at −3 EV in the mask.
module-toneequal-control-highlights-label = −2 EV
module-toneequal-control-highlights-full = −2 EV, highlights
module-toneequal-control-highlights-promise = Brightens or darkens the highlights, the tones at −2 EV in the mask.
module-toneequal-control-whites-label = −1 EV
module-toneequal-control-whites-full = −1 EV, whites
module-toneequal-control-whites-promise = Brightens or darkens the whites, the tones at −1 EV in the mask.
module-toneequal-control-speculars-label = +0 EV
module-toneequal-control-speculars-full = +0 EV, speculars
module-toneequal-control-speculars-promise = Brightens or darkens the speculars, the tones at 0 EV in the mask and above.
module-toneequal-control-smoothing-label = Curve smoothing
module-toneequal-control-smoothing-full = Curve smoothing
module-toneequal-control-smoothing-promise = Positive values give more progressive tone transitions, but the curve might oscillate in some settings. Negative values avoid oscillations and behave more robustly, but may give abrupt tone transitions and damage local contrast.
module-toneequal-control-exposure_boost-label = Mask exposure compensation
module-toneequal-control-exposure_boost-full = Mask exposure compensation
module-toneequal-control-exposure_boost-promise = Slides the mask's average exposure along the bands, for better control of the exposure correction with the bands there are.
module-toneequal-control-contrast_boost-label = Mask contrast compensation
module-toneequal-control-contrast_boost-full = Mask contrast compensation
module-toneequal-control-contrast_boost-promise = Counters the guided filter's averaging and dilates the mask's contrast around −4 EV, spreading its exposure histogram over more bands for better control of the exposure correction. Acts with the guided filter and EIGF only.
module-toneequal-control-method-label = Luminance estimator
module-toneequal-control-method-full = Luminance estimator
module-toneequal-control-method-promise = Preview the mask and choose the estimator that gives the higher contrast between areas to dodge and areas to burn.
module-toneequal-control-method-option-rgb_average = RGB average
module-toneequal-control-method-option-hsl_lightness = HSL lightness
module-toneequal-control-method-option-hsv_value = HSV value / RGB max
module-toneequal-control-method-option-rgb_sum = RGB sum
module-toneequal-control-method-option-rgb_euclidean_norm = RGB euclidean norm
module-toneequal-control-method-option-rgb_power_norm = RGB power norm
module-toneequal-control-method-option-rgb_geometric_mean = RGB geometric mean
module-toneequal-control-details-label = Preserve details
module-toneequal-control-details-full = Preserve details
module-toneequal-control-details-promise = No affects global and local contrast, safe if you only add contrast. The guided filter affects only global contrast and tries to preserve local contrast; the averaged guided filter is a geometric mean of no and the guided filter. EIGF, the exposure-independent guided filter, smooths shadows and highlights the same way, where the guided filter smooths the highlights less; averaged EIGF is a geometric mean of no and EIGF.
module-toneequal-control-details-option-none = No
module-toneequal-control-details-option-averaged_guided_filter = Averaged guided filter
module-toneequal-control-details-option-guided_filter = Guided filter
module-toneequal-control-details-option-averaged_eigf = Averaged EIGF
module-toneequal-control-details-option-eigf = EIGF
module-toneequal-control-iterations-label = Filter diffusion
module-toneequal-control-iterations-full = Filter diffusion
module-toneequal-control-iterations-promise = How many times the filter runs: more passes diffuse its edges, at the expense of speed.
module-toneequal-control-blending-label = Smoothing diameter
module-toneequal-control-blending-full = Smoothing diameter
module-toneequal-control-blending-promise = Diameter of the blur in percent of the image's largest side.
module-toneequal-control-feathering-label = Edges refinement/feathering
module-toneequal-control-feathering-full = Edges refinement/feathering
module-toneequal-control-feathering-promise = Precision of the feathering: higher values make the mask follow edges more closely but may void the smoothing; lower values give smoother gradients and better smoothing but may lead to inaccurate edges and halos.
module-toneequal-control-quantization-label = Mask quantisation
module-toneequal-control-quantization-full = Mask quantisation
module-toneequal-control-quantization-promise = 0 turns quantisation off. Higher values posterise the luminance mask, helping the filter produce piecewise smooth areas with high feathering values.
# darktable's display of the mask, a view mode no edit stores, drawn disabled until the
# mask preview comes.
module-toneequal-setting-display_exposure_mask-label = Display exposure mask
module-toneequal-setting-display_exposure_mask-full = Display exposure mask
module-toneequal-setting-display_exposure_mask-promise = Shows the mask the bands act on in place of the photo.

## Colour balance RGB (docs/modules/colour-balance-rgb.md): darktable's color balance rgb,
## its labels and tooltips in British English. A full name says which group a short label
## is in, as darktable's shortcuts do.

module-colorbalancergb-title = Colour balance RGB
module-colorbalancergb-tab-master = Master
module-colorbalancergb-tab-four_ways = 4 ways
module-colorbalancergb-tab-masks = Masks
module-colorbalancergb-group-linear_chroma_grading = Linear chroma grading
module-colorbalancergb-group-perceptual_saturation_grading = Perceptual saturation grading
module-colorbalancergb-group-perceptual_brilliance_grading = Perceptual brilliance grading
module-colorbalancergb-group-global_offset = Global offset
module-colorbalancergb-group-shadows_lift = Shadows lift
module-colorbalancergb-group-highlights_gain = Highlights gain
module-colorbalancergb-group-power = Power
module-colorbalancergb-group-luminance_ranges = Luminance ranges
module-colorbalancergb-group-threshold = Threshold
module-colorbalancergb-control-hue_angle-label = Hue shift
module-colorbalancergb-control-hue_angle-full = Hue shift
module-colorbalancergb-control-hue_angle-promise = Rotates all hues by an angle, at the same luminance.
module-colorbalancergb-control-vibrance-label = Global vibrance
module-colorbalancergb-control-vibrance-full = Global vibrance
module-colorbalancergb-control-vibrance-promise = Increases colourfulness mostly on low-chroma colours.
module-colorbalancergb-control-contrast-label = Contrast
module-colorbalancergb-control-contrast-full = Contrast
module-colorbalancergb-control-contrast-promise = Increases the contrast at constant chromaticity.
module-colorbalancergb-control-chroma_global-label = Global chroma
module-colorbalancergb-control-chroma_global-full = Global chroma
module-colorbalancergb-control-chroma_global-promise = Increases colourfulness at the same luminance, globally.
module-colorbalancergb-control-chroma_shadows-label = Shadows
module-colorbalancergb-control-chroma_shadows-full = Shadows chroma
module-colorbalancergb-control-chroma_shadows-promise = Increases colourfulness at the same luminance, mostly in the shadows.
module-colorbalancergb-control-chroma_midtones-label = Mid-tones
module-colorbalancergb-control-chroma_midtones-full = Mid-tones chroma
module-colorbalancergb-control-chroma_midtones-promise = Increases colourfulness at the same luminance, mostly in the mid-tones.
module-colorbalancergb-control-chroma_highlights-label = Highlights
module-colorbalancergb-control-chroma_highlights-full = Highlights chroma
module-colorbalancergb-control-chroma_highlights-promise = Increases colourfulness at the same luminance, mostly in the highlights.
module-colorbalancergb-control-saturation_global-label = Global saturation
module-colorbalancergb-control-saturation_global-full = Global saturation
module-colorbalancergb-control-saturation_global-promise = Adds or removes saturation by an absolute amount.
module-colorbalancergb-control-saturation_shadows-label = Shadows
module-colorbalancergb-control-saturation_shadows-full = Shadows saturation
module-colorbalancergb-control-saturation_shadows-promise = Increases or decreases saturation in proportion to each pixel's own, mostly in the shadows.
module-colorbalancergb-control-saturation_midtones-label = Mid-tones
module-colorbalancergb-control-saturation_midtones-full = Mid-tones saturation
module-colorbalancergb-control-saturation_midtones-promise = Increases or decreases saturation in proportion to each pixel's own, mostly in the mid-tones.
module-colorbalancergb-control-saturation_highlights-label = Highlights
module-colorbalancergb-control-saturation_highlights-full = Highlights saturation
module-colorbalancergb-control-saturation_highlights-promise = Increases or decreases saturation in proportion to each pixel's own, mostly in the highlights.
module-colorbalancergb-control-brilliance_global-label = Global brilliance
module-colorbalancergb-control-brilliance_global-full = Global brilliance
module-colorbalancergb-control-brilliance_global-promise = Adds or removes brilliance by an absolute amount.
module-colorbalancergb-control-brilliance_shadows-label = Shadows
module-colorbalancergb-control-brilliance_shadows-full = Shadows brilliance
module-colorbalancergb-control-brilliance_shadows-promise = Increases or decreases brilliance in proportion to each pixel's own, mostly in the shadows.
module-colorbalancergb-control-brilliance_midtones-label = Mid-tones
module-colorbalancergb-control-brilliance_midtones-full = Mid-tones brilliance
module-colorbalancergb-control-brilliance_midtones-promise = Increases or decreases brilliance in proportion to each pixel's own, mostly in the mid-tones.
module-colorbalancergb-control-brilliance_highlights-label = Highlights
module-colorbalancergb-control-brilliance_highlights-full = Highlights brilliance
module-colorbalancergb-control-brilliance_highlights-promise = Increases or decreases brilliance in proportion to each pixel's own, mostly in the highlights.
module-colorbalancergb-control-global_y-label = Luminance
module-colorbalancergb-control-global_y-full = Global offset luminance
module-colorbalancergb-control-global_y-promise = Global luminance offset.
module-colorbalancergb-control-global_h-label = Hue
module-colorbalancergb-control-global_h-full = Global offset hue
module-colorbalancergb-control-global_h-promise = Hue of the global colour offset.
module-colorbalancergb-control-global_c-label = Chroma
module-colorbalancergb-control-global_c-full = Global offset chroma
module-colorbalancergb-control-global_c-promise = Chroma of the global colour offset.
module-colorbalancergb-control-shadows_y-label = Luminance
module-colorbalancergb-control-shadows_y-full = Shadows lift luminance
module-colorbalancergb-control-shadows_y-promise = Luminance gain in the shadows.
module-colorbalancergb-control-shadows_h-label = Hue
module-colorbalancergb-control-shadows_h-full = Shadows lift hue
module-colorbalancergb-control-shadows_h-promise = Hue of the colour gain in the shadows.
module-colorbalancergb-control-shadows_c-label = Chroma
module-colorbalancergb-control-shadows_c-full = Shadows lift chroma
module-colorbalancergb-control-shadows_c-promise = Chroma of the colour gain in the shadows.
module-colorbalancergb-control-highlights_y-label = Luminance
module-colorbalancergb-control-highlights_y-full = Highlights gain luminance
module-colorbalancergb-control-highlights_y-promise = Luminance gain in the highlights.
module-colorbalancergb-control-highlights_h-label = Hue
module-colorbalancergb-control-highlights_h-full = Highlights gain hue
module-colorbalancergb-control-highlights_h-promise = Hue of the colour gain in the highlights.
module-colorbalancergb-control-highlights_c-label = Chroma
module-colorbalancergb-control-highlights_c-full = Highlights gain chroma
module-colorbalancergb-control-highlights_c-promise = Chroma of the colour gain in the highlights.
module-colorbalancergb-control-midtones_y-label = Luminance
module-colorbalancergb-control-midtones_y-full = Power luminance
module-colorbalancergb-control-midtones_y-promise = Luminance exponent in the mid-tones.
module-colorbalancergb-control-midtones_h-label = Hue
module-colorbalancergb-control-midtones_h-full = Power hue
module-colorbalancergb-control-midtones_h-promise = Hue of the colour exponent in the mid-tones.
module-colorbalancergb-control-midtones_c-label = Chroma
module-colorbalancergb-control-midtones_c-full = Power chroma
module-colorbalancergb-control-midtones_c-promise = Chroma of the colour exponent in the mid-tones.
module-colorbalancergb-control-saturation_formula-label = Saturation formula
module-colorbalancergb-control-saturation_formula-full = Saturation formula
module-colorbalancergb-control-saturation_formula-promise = Chooses the uniform colour space saturation is computed in.
module-colorbalancergb-control-saturation_formula-option-jzazbz = JzAzBz (2021)
module-colorbalancergb-control-saturation_formula-option-ucs = UCS (2022)
module-colorbalancergb-control-shadows_weight-label = Shadows fall-off
module-colorbalancergb-control-shadows_weight-full = Shadows fall-off
module-colorbalancergb-control-shadows_weight-promise = Weight of the shadows over the whole tonal range.
module-colorbalancergb-control-mask_grey_fulcrum-label = Mask middle-grey fulcrum
module-colorbalancergb-control-mask_grey_fulcrum-full = Mask middle-grey fulcrum
module-colorbalancergb-control-mask_grey_fulcrum-promise = Position of the middle-grey reference for masking.
module-colorbalancergb-control-highlights_weight-label = Highlights fall-off
module-colorbalancergb-control-highlights_weight-full = Highlights fall-off
module-colorbalancergb-control-highlights_weight-promise = Weight of the highlights over the whole tonal range.
module-colorbalancergb-control-white_fulcrum-label = White fulcrum
module-colorbalancergb-control-white_fulcrum-full = White fulcrum
module-colorbalancergb-control-white_fulcrum-promise = Peak white luminance used to normalise the power function.
module-colorbalancergb-control-grey_fulcrum-label = Contrast grey fulcrum
module-colorbalancergb-control-grey_fulcrum-full = Contrast grey fulcrum
module-colorbalancergb-control-grey_fulcrum-promise = Peak grey luminance used to normalise the power function: the grey Contrast keeps.

## Tone curve (docs/modules/tone-curve.md)

module-tone_curve-title = Tone curve
module-tone_curve-tab-luma = Luma
module-tone_curve-tab-rgb = RGB
module-tone_curve-tab-red = Red
module-tone_curve-tab-green = Green
module-tone_curve-tab-blue = Blue
module-tone_curve-tab-advanced = Advanced
module-tone_curve-control-luma_points-label = Luma
# The curve's name for screen readers.
module-tone_curve-control-luma_points-full = Tone curve, Luma
module-tone_curve-control-luma_points-promise = Moves each tone to the height you draw it at. Keeps colours. A straight line = untouched.
module-tone_curve-control-preserve_colour-label = Preserve colours
module-tone_curve-control-preserve_colour-full = Preserve colours in the Luma curve
module-tone_curve-control-preserve_colour-promise = What the Luma curve acts on in each pixel, scaling its three channels alike so its colour stays.
module-tone_curve-control-preserve_colour-option-luminance = Luminance
module-tone_curve-control-preserve_colour-option-max_rgb = Max RGB
module-tone_curve-control-preserve_colour-option-average_rgb = Average RGB
module-tone_curve-control-preserve_colour-option-sum_rgb = Sum RGB
module-tone_curve-control-preserve_colour-option-norm_rgb = Norm RGB
module-tone_curve-control-preserve_colour-option-basic_power = Basic power

## Haze removal (docs/modules/haze-removal.md): darktable's haze removal, its labels and
## tooltips in British English.

module-hazeremoval-title = Haze removal
module-hazeremoval-control-strength-label = Strength
module-hazeremoval-control-strength-full = Strength
module-hazeremoval-control-strength-promise = Amount of haze reduction.
module-hazeremoval-control-distance-label = Distance
module-hazeremoval-control-distance-full = Distance
module-hazeremoval-control-distance-promise = Limit haze removal up to a specific spatial depth.

## RGB primaries (docs/modules/rgb-primaries.md): darktable's rgb primaries, its labels
## and tooltips in British English.

module-primaries-title = RGB primaries
module-primaries-control-red_hue-label = Red hue
module-primaries-control-red_hue-full = Red hue
module-primaries-control-red_hue-promise = Shifts red towards yellow or magenta.
module-primaries-control-red_purity-label = Red purity
module-primaries-control-red_purity-full = Red purity
module-primaries-control-red_purity-promise = Red primary purity.
module-primaries-control-green_hue-label = Green hue
module-primaries-control-green_hue-full = Green hue
module-primaries-control-green_hue-promise = Shifts green towards cyan or yellow.
module-primaries-control-green_purity-label = Green purity
module-primaries-control-green_purity-full = Green purity
module-primaries-control-green_purity-promise = Green primary purity.
module-primaries-control-blue_hue-label = Blue hue
module-primaries-control-blue_hue-full = Blue hue
module-primaries-control-blue_hue-promise = Shifts blue towards magenta or cyan.
module-primaries-control-blue_purity-label = Blue purity
module-primaries-control-blue_purity-full = Blue purity
module-primaries-control-blue_purity-promise = Blue primary purity.
module-primaries-control-achromatic_tint_hue-label = Tint hue
module-primaries-control-achromatic_tint_hue-full = Tint hue
module-primaries-control-achromatic_tint_hue-promise = Tint hue.
module-primaries-control-achromatic_tint_purity-label = Tint purity
module-primaries-control-achromatic_tint_purity-full = Tint purity
module-primaries-control-achromatic_tint_purity-promise = Tint purity.

## Grain (docs/modules/grain.md): darktable's grain, its labels and tooltips in British
## English.

module-grain-title = Grain
module-grain-control-scale-label = Coarseness
module-grain-control-scale-full = Coarseness
module-grain-control-scale-promise = The grain size (~ISO of the film).
module-grain-control-strength-label = Strength
module-grain-control-strength-full = Strength
module-grain-control-strength-promise = The strength of applied grain.
module-grain-control-midtones_bias-label = Mid-tones bias
module-grain-control-midtones_bias-full = Mid-tones bias
module-grain-control-midtones_bias-promise = Amount of mid-tones bias from the photographic paper response modelling. The greater the bias, the more pronounced the fall-off of the grain in shadows and highlights.

## Soften (docs/modules/soften.md): darktable's soften, its labels and tooltips in
## British English.

module-soften-title = Soften
module-soften-control-size-label = Size
module-soften-control-size-full = Size
module-soften-control-size-promise = The size of blur.
module-soften-control-saturation-label = Saturation
module-soften-control-saturation-full = Saturation
module-soften-control-saturation-promise = The saturation of blur.
module-soften-control-brightness-label = Brightness
module-soften-control-brightness-full = Brightness
module-soften-control-brightness-promise = The brightness of blur.
module-soften-control-amount-label = Mix
module-soften-control-amount-full = Mix
module-soften-control-amount-promise = The mix of effect.

## Vignetting (docs/modules/vignetting.md): darktable's vignetting, its labels and tooltips
## in British English.

module-vignette-title = Vignetting
module-vignette-control-scale-label = Fall-off start
module-vignette-control-scale-full = Fall-off start
module-vignette-control-scale-promise = The radii scale of vignette for start of fall-off.
module-vignette-control-falloff_scale-label = Fall-off radius
module-vignette-control-falloff_scale-full = Fall-off radius
module-vignette-control-falloff_scale-promise = The radii scale of vignette for end of fall-off.
module-vignette-control-brightness-label = Brightness
module-vignette-control-brightness-full = Brightness
module-vignette-control-brightness-promise = Strength of effect on brightness.
module-vignette-control-saturation-label = Saturation
module-vignette-control-saturation-full = Saturation
module-vignette-control-saturation-promise = Strength of effect on saturation.
module-vignette-group-position_form = Position / form
module-vignette-control-center_x-label = Horizontal centre
module-vignette-control-center_x-full = Horizontal centre
module-vignette-control-center_x-promise = Horizontal offset of centre of the effect.
module-vignette-control-center_y-label = Vertical centre
module-vignette-control-center_y-full = Vertical centre
module-vignette-control-center_y-promise = Vertical offset of centre of the effect.
module-vignette-control-shape-label = Shape
module-vignette-control-shape-full = Shape
module-vignette-control-shape-promise = Shape factor: 0 produces a rectangle, 1 a circle or ellipse, 2 a diamond.
module-vignette-control-autoratio-label = Automatic ratio
module-vignette-control-autoratio-full = Automatic ratio
module-vignette-control-autoratio-promise = Enable to have the ratio automatically follow the image size.
module-vignette-control-whratio-label = Width/height ratio
module-vignette-control-whratio-full = Width/height ratio
module-vignette-control-whratio-promise = Width-to-height ratio.
module-vignette-control-dithering-label = Dithering
module-vignette-control-dithering-full = Dithering
module-vignette-control-dithering-promise = Add some level of random noise to prevent banding.
module-vignette-control-dithering-option-off = Off
module-vignette-control-dithering-option-output_8bit = 8-bit output
module-vignette-control-dithering-option-output_16bit = 16-bit output
