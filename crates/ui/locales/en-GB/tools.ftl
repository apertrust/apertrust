# The tools' words in British English, the source language (docs/gui.md, Tool panels
# from definitions). Each id follows from the tool's definition in apertrust-tools:
#   tool-<tool>-title, tool-<tool>-description, tool-<tool>-auto, tool-<tool>-tab-<tab>,
#   tool-<tool>-group-<group>, tool-<tool>-collapsible-<group>,
#   tool-<tool>-control-<control>-label, -full and -promise,
#   tool-<tool>-control-<control>-label-value where the tool gives a value of the
#   photo's after the label, tool-<tool>-control-<control>-option-<option>, with
#   -option-<option>-unavailable for a segmented choice's option a photo does not list,
#   a tool's setting's
#   tool-<tool>-setting-<setting>-label, -full, -promise and -option-<option>, and a
#   preset row's tool-<tool>-preset-<preset>-label, -full, -promise and
#   -option-<option>.
# Each title has a .search attribute, the other words Find Tool… finds the tool by (decision
# 96): darktable's name for its module and darktable's aliases for it, as darktable's
# strings and translations write them at the reference release, | between them; a
# utility's is on its section-<id> in ui.ftl. American English writes it again where it
# respells the title.
# Units are unit-<unit>, and a preset row's Custom preset-custom, in ui.ftl. A test in
# crates/tools/tests checks that every id a built-in tool implies is here; Ukrainian
# (uk) and Polish (pl) translate each one.

## Raw black/white point (docs/tools/raw-black-white-point.md): darktable's raw
## black/white point, its labels and tooltips in British English.

tool-rawprepare-title = Raw black/white point
    .search = raw black/white point
tool-rawprepare-description = Sets where black and white lie in the camera’s raw data, as the sensor recorded them. Usually left as it is: change it only with great care.
tool-rawprepare-control-raw_black_level_separate_0-label = Black level 0
tool-rawprepare-control-raw_black_level_separate_0-full = Black level 0
tool-rawprepare-control-raw_black_level_separate_0-promise = Black level 0.
tool-rawprepare-control-raw_black_level_separate_1-label = Black level 1
tool-rawprepare-control-raw_black_level_separate_1-full = Black level 1
tool-rawprepare-control-raw_black_level_separate_1-promise = Black level 1.
tool-rawprepare-control-raw_black_level_separate_2-label = Black level 2
tool-rawprepare-control-raw_black_level_separate_2-full = Black level 2
tool-rawprepare-control-raw_black_level_separate_2-promise = Black level 2.
tool-rawprepare-control-raw_black_level_separate_3-label = Black level 3
tool-rawprepare-control-raw_black_level_separate_3-full = Black level 3
tool-rawprepare-control-raw_black_level_separate_3-promise = Black level 3.
tool-rawprepare-control-raw_white_point-label = White point
tool-rawprepare-control-raw_white_point-full = White point
tool-rawprepare-control-raw_white_point-promise = White point.
tool-rawprepare-control-flat_field-label = Flat field correction
tool-rawprepare-control-flat_field-full = Flat field correction
tool-rawprepare-control-flat_field-promise = Raw flat field correction to compensate for lens shading.
tool-rawprepare-control-flat_field-option-off = Disabled
tool-rawprepare-control-flat_field-option-embedded = Embedded GainMap

## Highlight reconstruction (docs/tools/highlight-reconstruction.md): darktable's
## highlight reconstruction, its labels and tooltips in British English.

tool-highlights-title = Highlight reconstruction
    .search = highlight reconstruction
tool-highlights-description = Keeps clipped highlights from turning magenta and tries to recover their colour. Reach for it when the brightest areas look flat or tinted.
tool-highlights-control-mode-label = Method
tool-highlights-control-mode-full = Method
tool-highlights-control-mode-promise = Highlight reconstruction method.
tool-highlights-control-mode-option-opposed = Inpaint opposed
tool-highlights-control-mode-option-lch = Reconstruct in LCh
tool-highlights-control-mode-option-clip = Clip highlights
tool-highlights-control-mode-option-segments = Segmentation based
tool-highlights-control-mode-option-laplacian = Guided laplacians
tool-highlights-control-clip-label = Clipping threshold
tool-highlights-control-clip-full = Clipping threshold
tool-highlights-control-clip-promise = Manually adjust the clipping threshold mostly used against magenta highlights. You might use this for tuning 'Laplacian', 'Inpaint opposed' or 'Segmentation' modes, especially if camera white point is incorrect.
tool-highlights-control-combine-label = Combine
tool-highlights-control-combine-full = Combine
tool-highlights-control-combine-promise = Combine closely related clipped segments by morphological operations. This often leads to improved colour reconstruction for tiny segments before dark background.
tool-highlights-control-candidating-label = Candidating
tool-highlights-control-candidating-full = Candidating
tool-highlights-control-candidating-promise = Select inpainting after segmentation analysis. Increase to favour candidates found in segmentation analysis, decrease for opposed means inpainting.
tool-highlights-control-recovery-label = Rebuild
tool-highlights-control-recovery-full = Rebuild
tool-highlights-control-recovery-promise = Approximate lost data in regions with all photosites clipped, the effect depends on segment size and border gradients. Choose a mode tuned for segment size or the generic mode that tries to find best settings for every segment. Small means areas with a diameter less than 25 pixels, large is best for greater than 100. The flat modes ignore narrow unclipped structures (like power lines) to keep highlights rebuilt and avoid gradients.
tool-highlights-control-recovery-option-off = Off
tool-highlights-control-recovery-option-generic = Generic
tool-highlights-control-recovery-option-flat_generic = Flat generic
tool-highlights-control-recovery-option-small_segments = Small segments
tool-highlights-control-recovery-option-large_segments = Large segments
tool-highlights-control-recovery-option-flat_small_segments = Flat small segments
tool-highlights-control-recovery-option-flat_large_segments = Flat large segments
tool-highlights-control-strength-label = Strength
tool-highlights-control-strength-full = Strength
tool-highlights-control-strength-promise = Set strength of rebuilding in regions with all photosites clipped.
tool-highlights-control-noise_level-label = Noise level
tool-highlights-control-noise_level-full = Noise level
tool-highlights-control-noise_level-promise = Add noise to visually blend the reconstructed areas into the rest of the noisy image. Useful at high ISO.
tool-highlights-control-iterations-label = Iterations
tool-highlights-control-iterations-full = Iterations
tool-highlights-control-iterations-promise = Increase if magenta highlights don't get fully corrected. Each new iteration brings a performance penalty.
tool-highlights-control-solid_color-label = Inpaint a flat colour
tool-highlights-control-solid_color-full = Inpaint a flat colour
tool-highlights-control-solid_color-promise = Increase if magenta highlights don't get fully corrected. This may produce non-smooth boundaries between valid and clipped regions.
tool-highlights-control-scales-label = Diameter of reconstruction
tool-highlights-control-scales-full = Diameter of reconstruction
tool-highlights-control-scales-promise = Increase to correct larger clipped areas. Large values bring huge performance penalties.
tool-highlights-control-scales-option-px_2 = 2 px
tool-highlights-control-scales-option-px_4 = 4 px
tool-highlights-control-scales-option-px_8 = 8 px
tool-highlights-control-scales-option-px_16 = 16 px
tool-highlights-control-scales-option-px_32 = 32 px
tool-highlights-control-scales-option-px_64 = 64 px
tool-highlights-control-scales-option-px_128 = 128 px (slow)
tool-highlights-control-scales-option-px_256 = 256 px (slow)
tool-highlights-control-scales-option-px_512 = 512 px (very slow)
tool-highlights-control-scales-option-px_1024 = 1024 px (very slow)
tool-highlights-control-scales-option-px_2048 = 2048 px (insanely slow)
tool-highlights-control-scales-option-px_4096 = 4096 px (insanely slow)

## Demosaic (docs/tools/demosaic.md): darktable's demosaic, its labels and tooltips in
## British English, the method's two tooltips, for Bayer and X-Trans sensors, in one.

tool-demosaic-title = Demosaic
    .search = demosaic
tool-demosaic-description = Turns the sensor’s mosaic of red, green and blue into full-colour pixels. Usually left as it is: change it when fine detail shows mazes or false colour.
tool-demosaic-control-demosaicing_method-label = Method
tool-demosaic-control-demosaicing_method-full = Demosaicing method
tool-demosaic-control-demosaicing_method-promise = Demosaicing method. For a Bayer sensor PPG and RCD are fast, AMaZE and LMMSE are slow; LMMSE is suited best for high ISO images; VNG4 is good in rare cases avoiding maze patterns. For an X-Trans sensor Markesteijn 3-pass and frequency domain chroma are slow. Dual demosaicers increase processing time by blending a VNG variant in a second pass.
tool-demosaic-control-demosaicing_method-option-ppg = PPG
tool-demosaic-control-demosaicing_method-option-amaze = AMaZE
tool-demosaic-control-demosaicing_method-option-vng4 = VNG4
tool-demosaic-control-demosaicing_method-option-rcd = RCD
tool-demosaic-control-demosaicing_method-option-lmmse = LMMSE
tool-demosaic-control-demosaicing_method-option-rcd_dual = RCD (dual)
tool-demosaic-control-demosaicing_method-option-amaze_dual = AMaZE (dual)
tool-demosaic-control-demosaicing_method-option-passthrough_monochrome = Passthrough (monochrome)
tool-demosaic-control-demosaicing_method-option-photosite_color = Photosite colour (debug)
tool-demosaic-control-demosaicing_method-option-vng = VNG
tool-demosaic-control-demosaicing_method-option-markesteijn = Markesteijn 1-pass
tool-demosaic-control-demosaicing_method-option-markesteijn_3 = Markesteijn 3-pass
tool-demosaic-control-demosaicing_method-option-fdc = Frequency domain chroma
tool-demosaic-control-demosaicing_method-option-markesteijn_3_dual = Markesteijn 3-pass (dual)
tool-demosaic-control-demosaicing_method-option-passthrough_monochrome_xtrans = Passthrough (monochrome)
tool-demosaic-control-demosaicing_method-option-photosite_color_xtrans = Photosite colour (debug)
tool-demosaic-control-demosaicing_method-option-monochrome = Monochrome
tool-demosaic-control-dual_thrs-label = Dual threshold
tool-demosaic-control-dual_thrs-full = Dual threshold
tool-demosaic-control-dual_thrs-promise = Contrast threshold for dual demosaic: 0.0 for high frequency content, 1.0 for flat content.
tool-demosaic-control-median_thrs-label = Edge threshold
tool-demosaic-control-median_thrs-full = Edge threshold
tool-demosaic-control-median_thrs-promise = Threshold for edge-aware median: 0.0 switches it off, 1.0 ignores edges.
tool-demosaic-control-lmmse_refine-label = LMMSE refine
tool-demosaic-control-lmmse_refine-full = LMMSE refine
tool-demosaic-control-lmmse_refine-promise = LMMSE refinement steps. The median steps average the output; refine adds some recalculation of red and blue channels.
tool-demosaic-control-lmmse_refine-option-basic = Basic
tool-demosaic-control-lmmse_refine-option-median = Median
tool-demosaic-control-lmmse_refine-option-median_3x = 3x median
tool-demosaic-control-lmmse_refine-option-refine_medians = Refine & medians
tool-demosaic-control-lmmse_refine-option-refine_2x_medians = 2x refine + medians
tool-demosaic-control-color_smoothing-label = Colour smoothing
tool-demosaic-control-color_smoothing-full = Colour smoothing
tool-demosaic-control-color_smoothing-promise = How many colour smoothing median steps after demosaicing.
tool-demosaic-control-color_smoothing-option-off = Disabled
tool-demosaic-control-color_smoothing-option-once = Once
tool-demosaic-control-color_smoothing-option-twice = Twice
tool-demosaic-control-color_smoothing-option-three = Three times
tool-demosaic-control-color_smoothing-option-four = Four times
tool-demosaic-control-color_smoothing-option-five = Five times
tool-demosaic-control-green_eq-label = Match greens
tool-demosaic-control-green_eq-full = Match greens
tool-demosaic-control-green_eq-promise = Green channels matching method. Only applies for some old sensors.
tool-demosaic-control-green_eq-option-no = Disabled
tool-demosaic-control-green_eq-option-local = Local average
tool-demosaic-control-green_eq-option-full = Full average
tool-demosaic-control-green_eq-option-both = Full and local average
tool-demosaic-control-cs_enabled-label = Capture sharpen
tool-demosaic-control-cs_enabled-full = Capture sharpen
tool-demosaic-control-cs_enabled-promise = Capture sharpen recovers details lost due to in-camera blurring, which can be caused by diffraction, the anti-aliasing filter or other sources of Gaussian-type blur.
tool-demosaic-collapsible-capture_sharpen_controls = Capture sharpen controls
tool-demosaic-control-cs_iter-label = Iterations
tool-demosaic-control-cs_iter-full = Capture sharpen iterations
tool-demosaic-control-cs_iter-promise = Set effect strength by iterations.
tool-demosaic-control-cs_radius-label = Radius
tool-demosaic-control-cs_radius-full = Capture sharpen radius
tool-demosaic-control-cs_radius-promise = Capture sharpen radius should reflect the overall Gaussian-type blur of the camera sensor, possibly the anti-aliasing filter and the lens. Increasing this too far will soon lead to artefacts like halos and ringing, especially when used with a large iterations setting. A radius set to zero will be recalculated automatically the next run; use for presets.
tool-demosaic-control-cs_thrs-label = Contrast sensitivity
tool-demosaic-control-cs_thrs-full = Capture sharpen contrast sensitivity
tool-demosaic-control-cs_thrs-promise = Restrict capture sharpening to areas with high local contrast: increase to exclude flat areas in very dark or noisy images, decrease for well exposed and low noise images. A threshold set to zero will be reset to defaults the next run; use for presets.
tool-demosaic-control-cs_boost-label = Corner boost
tool-demosaic-control-cs_boost-full = Capture sharpen corner boost
tool-demosaic-control-cs_boost-promise = Further increase sharpen radius at image corners; the sharp centre of the image will not be affected.
tool-demosaic-control-cs_center-label = Sharp centre
tool-demosaic-control-cs_center-full = Capture sharpen sharp centre
tool-demosaic-control-cs_center-promise = Adjust to the sharp image centre.

## AgX (docs/tools/agx.md): darktable's AgX, its labels and tooltips in British English.
## A full name says which group a short label is in, as darktable's shortcuts do.

tool-agx-title = AgX
    .search = AgX|tone mapping|view transform|display transform
tool-agx-description = Fits the light the camera caught into what a screen or a print can show, with natural highlights and colours. The default tone mapper, inspired by Blender’s AgX.
tool-agx-tab-settings = Settings
tool-agx-tab-primaries = Primaries
tool-agx-group-input_exposure_range = Input exposure range
tool-agx-group-basic_curve_parameters = Basic curve parameters
tool-agx-group-before_tone_mapping = Before tone mapping
tool-agx-group-after_tone_mapping = After tone mapping
tool-agx-collapsible-show_curve = Show curve
tool-agx-collapsible-advanced_curve_parameters = Advanced curve parameters
tool-agx-collapsible-look = Look
tool-agx-control-range_white_relative_ev-label = White relative exposure
tool-agx-control-range_white_relative_ev-full = White relative exposure
tool-agx-control-range_white_relative_ev-promise = Relative exposure above mid-grey (white point).
tool-agx-control-range_black_relative_ev-label = Black relative exposure
tool-agx-control-range_black_relative_ev-full = Black relative exposure
tool-agx-control-range_black_relative_ev-promise = Relative exposure below mid-grey (black point).
tool-agx-control-dynamic_range_scaling-label = Dynamic range scaling
tool-agx-control-dynamic_range_scaling-full = Dynamic range scaling
tool-agx-control-dynamic_range_scaling-promise = Symmetrically increase or decrease the computed dynamic range. Useful to give a safety margin to extreme luminances.
# darktable's labelled line with its picker setting both exposures, a row of pickers.
tool-agx-group-auto_tune_levels = Auto tune levels
tool-agx-picker-range-label = Auto tune levels
tool-agx-picker-range-full = Auto tune levels: pick from the photo
tool-agx-picker-range-promise = Sets black and white relative exposure using the selected area.
tool-agx-action-read_exposure-label = Read exposure
tool-agx-action-read_exposure-full = Read exposure
tool-agx-action-read_exposure-promise = Read exposure from the metadata and the Exposure tool.
tool-agx-control-curve_pivot_x-label = Pivot relative exposure
tool-agx-control-curve_pivot_x-full = Pivot relative exposure
tool-agx-control-curve_pivot_x-promise = Set the pivot's input exposure in EV relative to mid-grey.
tool-agx-control-curve_pivot_y_linear_output-label = Pivot target output
tool-agx-control-curve_pivot_y_linear_output-full = Pivot target output
tool-agx-control-curve_pivot_y_linear_output-promise = Darken or brighten the pivot (linear output power).
# The pickers on the quads: darktable's quad tooltips.
tool-agx-picker-white-full = White relative exposure: pick from the photo
tool-agx-picker-white-promise = Picks the white point.
tool-agx-picker-black-full = Black relative exposure: pick from the photo
tool-agx-picker-black-promise = Picks the black point.
tool-agx-picker-pivot-full = Pivot relative exposure: pick from the photo
tool-agx-picker-pivot-promise = Sets the pivot relative to mid-grey from the average luminance of the selected area.
tool-agx-picker-pivot_output-full = Pivot target output: pick from the photo
tool-agx-picker-pivot_output-promise = Sets the pivot relative to mid-grey from the average luminance of the selected area, and adjusts the output by the default mid-grey to mid-grey mapping.
tool-agx-control-curve_contrast_around_pivot-label = Contrast
tool-agx-control-curve_contrast_around_pivot-full = Curve contrast
tool-agx-control-curve_contrast_around_pivot-promise = Slope of the linear section around the pivot.
tool-agx-control-curve_shoulder_power-label = Shoulder power
tool-agx-control-curve_shoulder_power-full = Shoulder power
tool-agx-control-curve_shoulder_power-promise = Contrast in highlights. Higher values keep the slope nearly constant for longer, at the cost of a more sudden drop near white.
tool-agx-control-curve_toe_power-label = Toe power
tool-agx-control-curve_toe_power-full = Toe power
tool-agx-control-curve_toe_power-promise = Contrast in shadows. Higher values keep the slope nearly constant for longer, at the cost of a more sudden drop near black.
tool-agx-warning-shoulder =
    Shoulder power cannot be applied because the curve has lost its S shape
    due to the current settings for white relative exposure, contrast and pivot.
    To re-enable it, do one of the following:
    – increase contrast
    – increase pivot target output
    – increase white relative exposure
    – increase curve y gamma (in the Advanced curve parameters section)
tool-agx-warning-toe =
    Toe power cannot be applied because the curve has lost its S shape
    due to the current settings for black relative exposure, contrast and pivot.
    To re-enable it, do one of the following:
    – increase contrast
    – decrease pivot target output
    – decrease black relative exposure (make it more negative)
    – decrease curve y gamma (in the Advanced curve parameters section)
tool-agx-control-curve_linear_ratio_above_pivot-label = Shoulder start
tool-agx-control-curve_linear_ratio_above_pivot-full = Shoulder start
tool-agx-control-curve_linear_ratio_above_pivot-promise = Length to keep the curve linear above the pivot. May clip highlights.
tool-agx-control-curve_target_display_white_ratio-label = Target white
tool-agx-control-curve_target_display_white_ratio-full = Target white
tool-agx-control-curve_target_display_white_ratio-promise = Maximum linear output power.
tool-agx-control-curve_linear_ratio_below_pivot-label = Toe start
tool-agx-control-curve_linear_ratio_below_pivot-full = Toe start
tool-agx-control-curve_linear_ratio_below_pivot-promise = Length to keep the curve linear below the pivot. May crush shadows.
tool-agx-control-curve_target_display_black_ratio-label = Target black
tool-agx-control-curve_target_display_black_ratio-full = Target black
tool-agx-control-curve_target_display_black_ratio-promise = Raise for a faded look.
tool-agx-control-auto_gamma-label = Keep the pivot on the diagonal
tool-agx-control-auto_gamma-full = Keep the pivot on the diagonal
tool-agx-control-auto_gamma-promise = Adjusts the gamma automatically, trying to make sure the curve always remains S-shaped (given that contrast is high enough), so the toe and shoulder controls remain effective.
tool-agx-control-curve_gamma-label = Curve y gamma
tool-agx-control-curve_gamma-full = Curve y gamma
tool-agx-control-curve_gamma-promise = Shifts the representation (but not the output power) of the pivot along the y axis of the curve. Immediate contrast around the pivot is not affected, but shadows and highlights are; you may have to counteract it with the contrast slider or with the toe and shoulder controls.
tool-agx-control-look_slope-label = Slope
tool-agx-control-look_slope-full = Look slope
tool-agx-control-look_slope-promise = Decrease or increase contrast and brightness.
tool-agx-control-look_lift-label = Lift
tool-agx-control-look_lift-full = Look lift
tool-agx-control-look_lift-promise = Deepen or lift shadows.
tool-agx-control-look_brightness-label = Brightness
tool-agx-control-look_brightness-full = Look brightness
tool-agx-control-look_brightness-promise = Increase or decrease brightness.
tool-agx-control-look_saturation-label = Saturation
tool-agx-control-look_saturation-full = Look saturation
tool-agx-control-look_saturation-promise = Decrease or increase saturation.
tool-agx-control-look_original_hue_mix_ratio-label = Preserve hue
tool-agx-control-look_original_hue_mix_ratio-full = Preserve hue
tool-agx-control-look_original_hue_mix_ratio-promise = Increase to bring hues closer to the original.
tool-agx-control-base_primaries-label = Base primaries
tool-agx-control-base_primaries-full = Base primaries
tool-agx-control-base_primaries-promise = Colour space primaries to use as the base for the adjustments below. Export profile uses the profile set in Output colour profile.
tool-agx-control-base_primaries-option-export_profile = Export profile
tool-agx-control-base_primaries-option-working_profile = Working profile
tool-agx-control-base_primaries-option-rec2020 = Rec2020
tool-agx-control-base_primaries-option-display_p3 = Display P3
tool-agx-control-base_primaries-option-adobe_rgb = Adobe RGB (compatible)
tool-agx-control-base_primaries-option-srgb = sRGB
tool-agx-control-disable_primaries_adjustments-label = Disable adjustments
tool-agx-control-disable_primaries_adjustments-full = Disable primaries adjustments
tool-agx-control-disable_primaries_adjustments-promise = Disable purity adjustments and rotations, only applying the curve. Note that those adjustments are at the heart of AgX: without them the results are almost always going to be worse, especially with bright, saturated lights (e.g. LEDs). Mainly intended to be used for experimenting.
tool-agx-action-reset_primaries-label = Reset primaries
tool-agx-action-reset_primaries-full = Reset primaries
tool-agx-action-reset_primaries-promise = Reset primaries to a predefined configuration.
tool-agx-group-reset_primaries = Reset primaries
tool-agx-action-primaries_blender_like-label = Blender-like
tool-agx-action-primaries_blender_like-full = Reset primaries: Blender-like
tool-agx-action-primaries_blender_like-promise = The primaries of Blender’s AgX, over Rec. 2020.
tool-agx-action-primaries_smooth-label = Smooth
tool-agx-action-primaries_smooth-full = Reset primaries: Smooth
tool-agx-action-primaries_smooth-promise = Sigmoid’s smooth primaries, over the working profile, their purity not restored.
tool-agx-action-primaries_unmodified-label = Unmodified
tool-agx-action-primaries_unmodified-full = Reset primaries: Unmodified
tool-agx-action-primaries_unmodified-promise = Rec. 2020’s primaries as they are, with no attenuation, rotation or purity boost.
tool-agx-control-red_inset-label = Red attenuation
tool-agx-control-red_inset-full = Red attenuation
tool-agx-control-red_inset-promise = Increase to desaturate reds in highlights faster.
tool-agx-control-red_rotation-label = Red rotation
tool-agx-control-red_rotation-full = Red rotation
tool-agx-control-red_rotation-promise = Shift the red primary towards yellow (+) or magenta (−).
tool-agx-control-green_inset-label = Green attenuation
tool-agx-control-green_inset-full = Green attenuation
tool-agx-control-green_inset-promise = Increase to desaturate greens in highlights faster.
tool-agx-control-green_rotation-label = Green rotation
tool-agx-control-green_rotation-full = Green rotation
tool-agx-control-green_rotation-promise = Shift the green primary towards cyan (+) or yellow (−).
tool-agx-control-blue_inset-label = Blue attenuation
tool-agx-control-blue_inset-full = Blue attenuation
tool-agx-control-blue_inset-promise = Increase to desaturate blues in highlights faster.
tool-agx-control-blue_rotation-label = Blue rotation
tool-agx-control-blue_rotation-full = Blue rotation
tool-agx-control-blue_rotation-promise = Shift the blue primary towards magenta (+) or cyan (−).
tool-agx-control-completely_reverse_primaries-label = Reverse all
tool-agx-control-completely_reverse_primaries-full = Reverse all
tool-agx-control-completely_reverse_primaries-promise = Completely restore purity, undo all rotations, and hide the controls below. Uncheck to restore the previous state.
tool-agx-action-set_from_above-label = Set from above
tool-agx-action-set_from_above-full = Set from above
tool-agx-action-set_from_above-promise = Set parameters to completely reverse primaries modifications, but allow subsequent editing.
tool-agx-control-master_outset_ratio-label = Master purity boost
tool-agx-control-master_outset_ratio-full = Master purity boost
tool-agx-control-master_outset_ratio-promise = Overall purity boost.
tool-agx-control-master_unrotation_ratio-label = Master rotation reversal
tool-agx-control-master_unrotation_ratio-full = Master rotation reversal
tool-agx-control-master_unrotation_ratio-promise = Overall unrotation ratio.
tool-agx-control-red_outset-label = Red purity boost
tool-agx-control-red_outset-full = Red purity boost
tool-agx-control-red_outset-promise = Restore the purity of red, mostly in mid-tones and shadows.
tool-agx-control-red_unrotation-label = Red reverse rotation
tool-agx-control-red_unrotation-full = Red reverse rotation
tool-agx-control-red_unrotation-promise = Reverse the colour shift in reds.
tool-agx-control-green_outset-label = Green purity boost
tool-agx-control-green_outset-full = Green purity boost
tool-agx-control-green_outset-promise = Restore the purity of green, mostly in mid-tones and shadows.
tool-agx-control-green_unrotation-label = Green reverse rotation
tool-agx-control-green_unrotation-full = Green reverse rotation
tool-agx-control-green_unrotation-promise = Reverse the colour shift in greens.
tool-agx-control-blue_outset-label = Blue purity boost
tool-agx-control-blue_outset-full = Blue purity boost
tool-agx-control-blue_outset-promise = Restore the purity of blue, mostly in mid-tones and shadows.
tool-agx-control-blue_unrotation-label = Blue reverse rotation
tool-agx-control-blue_unrotation-full = Blue reverse rotation
tool-agx-control-blue_unrotation-promise = Reverse the colour shift in blues.

## Input colour profile (docs/tools/input-colour-profile.md): darktable's input color
## profile, its labels and its profiles' names in British English. darktable's tooltips
## for the two profiles name the folders it loads ICC files from, which Apertrust does
## not read.

tool-colorin-title = Input colour profile
    .search = input color profile
tool-colorin-description = Takes the camera’s colours into the colour space every later tool works in. Usually left at the camera’s own matrix.
tool-colorin-control-type-label = Input profile
tool-colorin-control-type-full = Input profile
tool-colorin-control-type-promise = The colour profile the camera's colours are read in.
tool-colorin-control-type-option-embedded_matrix = Embedded matrix
tool-colorin-control-type-option-standard_matrix = Standard colour matrix
tool-colorin-control-type-option-srgb = sRGB
tool-colorin-control-type-option-adobe_rgb = Adobe RGB (compatible)
tool-colorin-control-type-option-lin_rec709 = Linear Rec709 RGB
tool-colorin-control-type-option-rec709 = Rec709 RGB
tool-colorin-control-type-option-lin_rec2020 = Linear Rec2020 RGB
tool-colorin-control-type-option-pq_rec2020 = PQ Rec2020 RGB
tool-colorin-control-type-option-hlg_rec2020 = HLG Rec2020 RGB
tool-colorin-control-type-option-pq_p3 = PQ P3 RGB
tool-colorin-control-type-option-hlg_p3 = HLG P3 RGB
tool-colorin-control-type-option-display_p3 = Display P3 RGB
tool-colorin-control-type-option-prophoto_rgb = Linear ProPhoto RGB
tool-colorin-control-type-option-xyz = Linear XYZ
tool-colorin-control-type-option-lab = Lab
tool-colorin-control-type-option-infrared = Linear infrared BGR
tool-colorin-control-type-option-brg = BRG (for testing)
tool-colorin-control-type_work-label = Working profile
tool-colorin-control-type_work-full = Working profile
tool-colorin-control-type_work-promise = The colour space every tool after this one works in.
tool-colorin-control-type_work-option-srgb = sRGB (web-safe)
tool-colorin-control-type_work-option-adobe_rgb = Adobe RGB (compatible)
tool-colorin-control-type_work-option-lin_rec709 = Linear Rec709 RGB
tool-colorin-control-type_work-option-rec709 = Rec709 RGB
tool-colorin-control-type_work-option-lin_rec2020 = Linear Rec2020 RGB
tool-colorin-control-type_work-option-pq_rec2020 = PQ Rec2020 RGB
tool-colorin-control-type_work-option-hlg_rec2020 = HLG Rec2020 RGB
tool-colorin-control-type_work-option-pq_p3 = PQ P3 RGB
tool-colorin-control-type_work-option-hlg_p3 = HLG P3 RGB
tool-colorin-control-type_work-option-display_p3 = Display P3 RGB
tool-colorin-control-type_work-option-prophoto_rgb = Linear ProPhoto RGB
tool-colorin-control-normalize-label = Gamut clipping
tool-colorin-control-normalize-full = Gamut clipping
tool-colorin-control-normalize-promise = Confine Lab values to gamut of RGB colour space.
tool-colorin-control-normalize-option-off = Off
tool-colorin-control-normalize-option-srgb = sRGB
tool-colorin-control-normalize-option-adobe_rgb = Adobe RGB (compatible)
tool-colorin-control-normalize-option-lin_rec709 = Linear Rec709 RGB
tool-colorin-control-normalize-option-lin_rec2020 = Linear Rec2020 RGB

## Output colour profile (docs/tools/output-colour-profile.md): darktable's output color
## profile, its label and its profiles' names in British English. darktable's tooltip
## names the folders it loads ICC files from, which Apertrust does not read yet.

tool-colorout-title = Output colour profile
    .search = output color profile
tool-colorout-description = Converts the finished picture into the colours of the file you export. Usually left at sRGB, which suits screens and the web.
tool-colorout-control-type-label = Export profile
tool-colorout-control-type-full = Export profile
tool-colorout-control-type-promise = The colour profile the image is exported in.
tool-colorout-control-type-option-srgb = sRGB (web-safe)
tool-colorout-control-type-option-adobe_rgb = Adobe RGB (compatible)
tool-colorout-control-type-option-lin_rec709 = Linear Rec709 RGB
tool-colorout-control-type-option-rec709 = Rec709 RGB
tool-colorout-control-type-option-lin_rec2020 = Linear Rec2020 RGB
tool-colorout-control-type-option-pq_rec2020 = PQ Rec2020 RGB
tool-colorout-control-type-option-hlg_rec2020 = HLG Rec2020 RGB
tool-colorout-control-type-option-pq_p3 = PQ P3 RGB
tool-colorout-control-type-option-hlg_p3 = HLG P3 RGB
tool-colorout-control-type-option-display_p3 = Display P3 RGB
tool-colorout-control-type-option-prophoto_rgb = Linear ProPhoto RGB
tool-colorout-control-type-option-brg = BRG (for testing)

## White balance (docs/tools/white-balance.md): darktable's white balance, its labels
## and tooltips in British English.

tool-temperature-title = White balance
    .search = white balance
tool-temperature-description = Balances the camera’s red, green and blue so that white comes out white, and helps demosaic. Usually left as it is: set the colour of the light in Colour calibration.
tool-temperature-preset-settings-label = Settings
tool-temperature-preset-settings-full = White balance settings
tool-temperature-preset-settings-promise = Chooses the white balance setting: the coefficients and where they come from.
tool-temperature-preset-settings-option-as_shot = As shot
tool-temperature-preset-settings-option-spot = From image area
tool-temperature-preset-settings-option-user = User modified
tool-temperature-preset-settings-option-d65 = Camera reference
tool-temperature-preset-settings-option-d65_late = As shot to reference
tool-temperature-control-preset-label = Settings
tool-temperature-control-preset-full = White balance settings
tool-temperature-control-preset-promise = Where the coefficients come from. As shot to reference sets white balance to as shot and later corrects it to the camera's reference point, in most cases D65.
tool-temperature-control-preset-option-as_shot = As shot
tool-temperature-control-preset-option-spot = From image area
tool-temperature-control-preset-option-user = User modified
tool-temperature-control-preset-option-d65 = Camera reference
tool-temperature-control-preset-option-d65_late = As shot to reference
tool-temperature-group-scene_illuminant_temp = Scene illuminant temp
tool-temperature-control-temperature-label = Temperature
tool-temperature-control-temperature-full = Scene illuminant temperature
tool-temperature-control-temperature-promise = The colour temperature, in kelvin.
tool-temperature-control-tint-label = Tint
tool-temperature-control-tint-full = Scene illuminant tint
tool-temperature-control-tint-promise = The colour tint of the image, from magenta (below 1) to green (above 1).
tool-temperature-collapsible-channel_coefficients = Channel coefficients
tool-temperature-control-red-label = Red
tool-temperature-control-red-full = Red coefficient
tool-temperature-control-red-promise = What the raw's red photosites are multiplied by.
tool-temperature-control-green-label = Green
tool-temperature-control-green-full = Green coefficient
tool-temperature-control-green-promise = What the raw's green photosites are multiplied by.
tool-temperature-control-blue-label = Blue
tool-temperature-control-blue-full = Blue coefficient
tool-temperature-control-blue-promise = What the raw's blue photosites are multiplied by.
tool-temperature-control-various-label = Emerald
tool-temperature-control-various-full = Emerald coefficient
tool-temperature-control-various-promise = What a four-colour sensor's emerald photosites are multiplied by.
# The picker, darktable's "from image area", which Settings' From image area turns on: its
# name in the viewer bar, and darktable's tooltip.
tool-temperature-picker-spot-label = From image area
tool-temperature-picker-spot-full = White balance from an image area
tool-temperature-picker-spot-promise = Set white balance to detected from area
# Its picker cursor on the Toolbar (decision 104): its name, and how to use it.
tool-temperature-cursor-spot = White balance picker
tool-temperature-cursor-spot-how = Click a neutral area, or drag one

## Orientation (docs/tools/orientation.md): darktable's orientation, its label, its
## buttons and its orientations by darktable's names in British English. darktable shows
## the stored orientation under its Transform label only as the photo itself.

tool-flip-title = Orientation
    .search = orientation|rotation|flip|mirror
tool-flip-description = Turns the photo a quarter turn at a time, or mirrors it. Reach for it when a photo shows on its side.
tool-flip-control-orientation-label = Transform
tool-flip-control-orientation-full = Transform
tool-flip-control-orientation-promise = Flip or rotate the image by steps of 90°.
tool-flip-control-orientation-option-auto = Autodetect
tool-flip-control-orientation-option-none = No rotation
tool-flip-control-orientation-option-flip_vertically = Flip vertically
tool-flip-control-orientation-option-flip_horizontally = Flip horizontally
tool-flip-control-orientation-option-rotate_180 = Rotate 180°
tool-flip-control-orientation-option-transpose = Transpose
tool-flip-control-orientation-option-rotate_cw_90 = Rotate −90°
tool-flip-control-orientation-option-rotate_ccw_90 = Rotate 90°
tool-flip-control-orientation-option-transverse = Transverse
tool-flip-action-rotate_ccw-label = Rotate 90° anticlockwise
tool-flip-action-rotate_ccw-full = Rotate 90° anticlockwise
    .toolbar = Rotate left
tool-flip-action-rotate_ccw-promise = Rotate the image 90° anticlockwise.
tool-flip-action-rotate_cw-label = Rotate 90° clockwise
tool-flip-action-rotate_cw-full = Rotate 90° clockwise
    .toolbar = Rotate right
tool-flip-action-rotate_cw-promise = Rotate the image 90° clockwise.
tool-flip-action-flip_horizontally-label = Flip horizontally
tool-flip-action-flip_horizontally-full = Flip horizontally
tool-flip-action-flip_horizontally-promise = Mirror the image left to right.
tool-flip-action-flip_vertically-label = Flip vertically
tool-flip-action-flip_vertically-full = Flip vertically
tool-flip-action-flip_vertically-promise = Mirror the image top to bottom.

## Exposure (docs/tools/exposure.md): darktable's exposure, its labels and tooltips in
## British English. The promises of Exposure and Compensate camera exposure are its
## invariants table's, Black level correction's the redesign's (decision 88).

tool-exposure-title = Exposure
    .search = exposure
tool-exposure-description = Brightens or darkens the photo as if you had exposed it differently in the camera, keeping its colours. Usually the first tool to reach for.
tool-exposure-control-mode-label = Mode
tool-exposure-control-mode-full = Mode
tool-exposure-control-mode-promise = Manual sets the exposure by hand. Automatic sets it from each photo's raw histogram, to even out the exposure of a series.
tool-exposure-control-mode-option-manual = Manual
tool-exposure-control-mode-option-automatic = Automatic
# The switch's Automatic, drawn disabled on an image that is not a raw (decision 88).
tool-exposure-control-mode-option-automatic-unavailable = Automatic needs a raw photo's histogram
tool-exposure-control-camera_bias-label = Compensate camera exposure
tool-exposure-control-camera_bias-full = Compensate camera exposure
tool-exposure-control-camera_bias-promise = Undoes the exposure compensation set on the camera.
# The camera's bias after the label, no brackets; $value is it with its unit, "−⅔ EV".
tool-exposure-control-camera_bias-label-value = { $value }
tool-exposure-control-compensate_hilite_pres-label = Highlight preservation mode
tool-exposure-control-compensate_hilite_pres-full = Highlight preservation mode
tool-exposure-control-compensate_hilite_pres-promise = Removes the camera's hidden exposure bias in HDR, highlight preservation, dynamic range or HLG tone mode. On an image with a bias, it needs tone mapping, such as sigmoid, to avoid blown-out highlights.
# The exposure the camera held back after the label, no brackets; $value is it with its
# unit, "+1.0 EV".
tool-exposure-control-compensate_hilite_pres-label-value = { $value }
tool-exposure-control-exposure-label = Exposure
tool-exposure-control-exposure-full = Exposure
tool-exposure-control-exposure-promise = Adjusts the exposure correction.
# Automatic mode's rows as redesigned (decision 88), the tooltips of the first two starting
# with darktable's name for them: Percentile and Target level.
tool-exposure-control-deflicker_percentile-label = Measure at
tool-exposure-control-deflicker_percentile-full = Measure at
tool-exposure-control-deflicker_percentile-promise = Percentile: the point of the raw histogram measured; 50 % is the median
tool-exposure-control-deflicker_target_level-label = Place it at
tool-exposure-control-deflicker_target_level-full = Place it at
tool-exposure-control-deflicker_target_level-promise = Target level: where that point lands, in EV below white
tool-exposure-computed-computed_exposure-label = This photo
tool-exposure-computed-computed_exposure-promise = The exposure automatic mode computed for this photo from its histogram
tool-exposure-control-black_level-label = Black level correction
tool-exposure-control-black_level-full = Black level correction
tool-exposure-control-black_level-promise = For lifting clipped blacks, not for deeper blacks: use AgX's black relative exposure
# Picker target, under Exposure, in place of darktable's area exposure mapping (decision
# 88): what the picker makes of the area picked, each option's tooltip saying what it does.
tool-exposure-setting-target-label = Picker target
tool-exposure-setting-target-full = Picker target
tool-exposure-setting-target-promise = What Exposure's picker makes of the area you pick.
tool-exposure-setting-target-option-middle_grey = Middle grey
tool-exposure-setting-target-option-lightness = Lightness
tool-exposure-setting-target-option-reference = Reference
tool-exposure-setting-target-option-middle_grey-promise = The picked area becomes a normal mid-tone (L 50 %)
tool-exposure-setting-target-option-lightness-promise = The picked area becomes the brightness you set
tool-exposure-setting-target-option-reference-promise = The picked area matches the one measured on another photo
# Lightness, under Picker target while Lightness is chosen.
tool-exposure-setting-lightness-label = Lightness
tool-exposure-setting-lightness-full = Target lightness
tool-exposure-setting-lightness-promise = The lightness the picked area is given, CIE L*
# Exposure's picker on its quad. Its tooltip and its hint over the photo say what a pick
# will do, by Picker target's option; $lightness is Lightness, $reference_lightness the
# reference's L, in the language's numbers, and $reference_photo the name of the photo it was
# measured on. The button's name is the same words, its action. With Reference chosen and
# none set, it is disabled, saying why.
tool-exposure-picker-exposure-promise-middle_grey = Make an area middle grey
tool-exposure-picker-exposure-hint-middle_grey = Make an area middle grey
tool-exposure-picker-exposure-promise-lightness = Make an area L { $lightness } %
tool-exposure-picker-exposure-hint-lightness = Make an area L { $lightness } %
tool-exposure-picker-exposure-promise-reference = Match an area to { $reference_photo } (L { $reference_lightness } %)
tool-exposure-picker-exposure-hint-reference = Match an area to { $reference_photo } (L { $reference_lightness } %)
# A reference whose photo is not known, as one --conf gives.
tool-exposure-picker-exposure-promise-reference-no-photo = Match an area to L { $reference_lightness } %
tool-exposure-picker-exposure-hint-reference-no-photo = Match an area to L { $reference_lightness } %
# A reference measured on the photo that is open.
tool-exposure-picker-exposure-promise-reference-this-photo = Match an area to this photo (L { $reference_lightness } %)
tool-exposure-picker-exposure-hint-reference-this-photo = Match an area to this photo (L { $reference_lightness } %)
tool-exposure-picker-exposure-unavailable = Set a reference first, with the picker on the Reference row
# Added to a short pick's tooltip where the pick moved Black level correction too.
tool-exposure-picker-exposure-short-black = Black level was moved to keep black below white
# Its picker cursor on the Toolbar (decision 104): its name; how to use it is the
# picker's promise for Picker target.
tool-exposure-cursor-exposure = Exposure picker
# The reference row's picker (decision 88): its label the row's, its name its action.
tool-exposure-picker-reference-label = Reference
tool-exposure-picker-reference-full = Set the reference from an area
tool-exposure-picker-reference-promise = Set the reference from an area
tool-exposure-picker-reference-hint = Drag over the area to keep as the reference
# Its picker cursor on the Toolbar (decision 104): its name and how to use it.
tool-exposure-cursor-reference = Exposure reference picker
tool-exposure-cursor-reference-how = Click an area to keep as the reference, or drag one

## Colour calibration (docs/tools/colour-calibration.md): darktable's color calibration,
## every tab, the CAT tab first. Its words are darktable's, the adaptation's and the area
## colour mapping's promises its tooltips; darktable gives its other CAT controls none, so
## their promises say what each describes.

tool-channelmixerrgb-title = Colour calibration
    .search = color calibration|channel mixer|white balance|monochrome
tool-channelmixerrgb-description = Sets the colour of the light, mixes the colour channels, or turns the photo black and white as film would. Reach for it to remove a colour cast.
tool-channelmixerrgb-tab-cat = CAT
tool-channelmixerrgb-control-adaptation-label = Adaptation
tool-channelmixerrgb-control-adaptation-full = Adaptation
tool-channelmixerrgb-control-adaptation-promise = How the illuminant is adapted, and the colour space the tool works in. Linear Bradford (1985) is consistent with the ICC v4 toolchain. CAT16 (2016) is more robust and accurate. Non-linear Bradford (1985) is the original Bradford: it can give better results than the linear one, but is unreliable. XYZ is a simple scaling in XYZ space, not recommended in general. None disables any adaptation and uses the pipeline's working RGB.
tool-channelmixerrgb-control-adaptation-option-linear_bradford = Linear Bradford (ICC v4)
tool-channelmixerrgb-control-adaptation-option-cat16 = CAT16 (CIECAM16)
tool-channelmixerrgb-control-adaptation-option-non_linear_bradford = Non-linear Bradford
tool-channelmixerrgb-control-adaptation-option-xyz = XYZ
tool-channelmixerrgb-control-adaptation-option-none = None (bypass)
tool-channelmixerrgb-control-illuminant-label = Illuminant
tool-channelmixerrgb-control-illuminant-full = Illuminant
tool-channelmixerrgb-control-illuminant-promise = The light the scene was lit by, which the adaptation turns into white: a standard light, one at a temperature, the camera's own or a custom colour.
tool-channelmixerrgb-control-illuminant-option-pipeline = Same as pipeline (D50)
tool-channelmixerrgb-control-illuminant-option-a = A (incandescent)
tool-channelmixerrgb-control-illuminant-option-d = D (daylight)
tool-channelmixerrgb-control-illuminant-option-e = E (equi-energy)
tool-channelmixerrgb-control-illuminant-option-f = F (fluorescent)
tool-channelmixerrgb-control-illuminant-option-led = LED (LED light)
tool-channelmixerrgb-control-illuminant-option-planckian = Planckian (black body)
tool-channelmixerrgb-control-illuminant-option-custom = Custom
tool-channelmixerrgb-control-illuminant-option-camera = As shot in camera
tool-channelmixerrgb-control-illum_fluo-label = F source
tool-channelmixerrgb-control-illum_fluo-full = Fluorescent source
tool-channelmixerrgb-control-illum_fluo-promise = Which CIE fluorescent light lit the scene.
tool-channelmixerrgb-control-illum_fluo-option-f1 = F1 (Daylight 6430 K) – medium CRI
tool-channelmixerrgb-control-illum_fluo-option-f2 = F2 (Cool White 4230 K) – medium CRI
tool-channelmixerrgb-control-illum_fluo-option-f3 = F3 (White 3450 K) – medium CRI
tool-channelmixerrgb-control-illum_fluo-option-f4 = F4 (Warm White 2940 K) – medium CRI
tool-channelmixerrgb-control-illum_fluo-option-f5 = F5 (Daylight 6350 K) – medium CRI
tool-channelmixerrgb-control-illum_fluo-option-f6 = F6 (Lite White 4150 K) – medium CRI
tool-channelmixerrgb-control-illum_fluo-option-f7 = F7 (D65 simulator 6500 K) – high CRI
tool-channelmixerrgb-control-illum_fluo-option-f8 = F8 (D50 simulator 5000 K) – high CRI
tool-channelmixerrgb-control-illum_fluo-option-f9 = F9 (Cool White Deluxe 4150 K) – high CRI
tool-channelmixerrgb-control-illum_fluo-option-f10 = F10 (Tuned RGB 5000 K) – low CRI
tool-channelmixerrgb-control-illum_fluo-option-f11 = F11 (Tuned RGB 4000 K) – low CRI
tool-channelmixerrgb-control-illum_fluo-option-f12 = F12 (Tuned RGB 3000 K) – low CRI
tool-channelmixerrgb-control-illum_led-label = LED source
tool-channelmixerrgb-control-illum_led-full = LED source
tool-channelmixerrgb-control-illum_led-promise = Which CIE LED light lit the scene.
tool-channelmixerrgb-control-illum_led-option-b1 = B1 (Blue 2733 K)
tool-channelmixerrgb-control-illum_led-option-b2 = B2 (Blue 2998 K)
tool-channelmixerrgb-control-illum_led-option-b3 = B3 (Blue 4103 K)
tool-channelmixerrgb-control-illum_led-option-b4 = B4 (Blue 5109 K)
tool-channelmixerrgb-control-illum_led-option-b5 = B5 (Blue 6598 K)
tool-channelmixerrgb-control-illum_led-option-bh1 = BH1 (Blue-Red hybrid 2851 K)
tool-channelmixerrgb-control-illum_led-option-rgb1 = RGB1 (RGB 2840 K)
tool-channelmixerrgb-control-illum_led-option-v1 = V1 (Violet 2724 K)
tool-channelmixerrgb-control-illum_led-option-v2 = V2 (Violet 4070 K)
tool-channelmixerrgb-control-temperature-label = Temperature
tool-channelmixerrgb-control-temperature-full = Illuminant temperature
tool-channelmixerrgb-control-temperature-promise = The temperature of the light, on the daylight locus or the black body's.
tool-channelmixerrgb-control-x-label = x
tool-channelmixerrgb-control-x-full = Illuminant x
tool-channelmixerrgb-control-x-promise = The light's CIE 1931 x, as the pipeline sees it. Hue and chroma show it.
tool-channelmixerrgb-control-y-label = y
tool-channelmixerrgb-control-y-full = Illuminant y
tool-channelmixerrgb-control-y-promise = The light's CIE 1931 y, as the pipeline sees it. Hue and chroma show it.
tool-channelmixerrgb-control-hue-label = Hue
tool-channelmixerrgb-control-hue-full = Illuminant hue
tool-channelmixerrgb-control-hue-promise = The colour of the light, around the pipeline's white.
tool-channelmixerrgb-control-chroma-label = Chroma
tool-channelmixerrgb-control-chroma-full = Illuminant chroma
tool-channelmixerrgb-control-chroma-promise = How far the light is from the pipeline's white. 0 = white.
tool-channelmixerrgb-control-gamut-label = Gamut compression
tool-channelmixerrgb-control-gamut-full = Gamut compression
tool-channelmixerrgb-control-gamut-promise = Brings colours outside the pipeline's gamut back inside it, the more the higher. 0 = none.
tool-channelmixerrgb-control-clip-label = Clip negative RGB from gamut
tool-channelmixerrgb-control-clip-full = Clip negative RGB from gamut
tool-channelmixerrgb-control-clip-promise = Clips the negative RGB values the adaptation leaves outside the gamut.
# The readout row, darktable's CCT, the illuminant's colour and its picker, on the CAT tab
# while the adaptation is not none.
tool-channelmixerrgb-readout-cct-daylight = CCT: { $value } K (daylight)
tool-channelmixerrgb-readout-cct-daylight-promise = Approximated correlated colour temperature. This illuminant can be accurately modelled by a daylight spectrum, so its temperature is relevant and meaningful with a D illuminant.
tool-channelmixerrgb-readout-cct-black_body = CCT: { $value } K (black body)
tool-channelmixerrgb-readout-cct-black_body-promise = Approximated correlated colour temperature. This illuminant can be accurately modelled by a black body spectrum, so its temperature is relevant and meaningful with a Planckian illuminant.
tool-channelmixerrgb-readout-cct-invalid = CCT: { $value } K (invalid)
tool-channelmixerrgb-readout-cct-invalid-promise = Approximated correlated colour temperature. This illuminant cannot be accurately modelled by a daylight or black body spectrum, so its temperature is not relevant and meaningful and you need to use a custom illuminant.
tool-channelmixerrgb-readout-cct-undefined = CCT: undefined
tool-channelmixerrgb-readout-cct-undefined-promise = The approximated correlated colour temperature cannot be computed at all, so you need to use a custom illuminant.
tool-channelmixerrgb-readout-cct-swatch = This is the colour of the scene illuminant before chromatic adaptation. This colour will be turned into pure white by the adaptation.
tool-channelmixerrgb-picker-illuminant-label = Illuminant from area
tool-channelmixerrgb-picker-illuminant-full = Illuminant from an image area
# Its picker cursor on the Toolbar (decision 104): its name and how to use it.
tool-channelmixerrgb-cursor-illuminant = Colour calibration picker
tool-channelmixerrgb-cursor-illuminant-how = Click an area, or drag one
tool-channelmixerrgb-picker-illuminant-promise = Set white balance to detected from area

# The area colour mapping's settings, darktable's own, kept in settings and never in an
# edit, which the picker reads.
tool-channelmixerrgb-collapsible-area_colour_mapping = Area colour mapping
tool-channelmixerrgb-setting-area_mode-label = Area mode
tool-channelmixerrgb-setting-area_mode-full = Area mode
tool-channelmixerrgb-setting-area_mode-promise = Correction adjusts the illuminant so the input colour is mapped to the target. Measure only shows how an input colour is mapped by the CAT, and can be used to sample a target.
tool-channelmixerrgb-setting-area_mode-option-correction = Correction
tool-channelmixerrgb-setting-area_mode-option-measure = Measure
tool-channelmixerrgb-setting-channel_mixing-label = Take channel mixing into account
tool-channelmixerrgb-setting-channel_mixing-full = Take channel mixing into account
tool-channelmixerrgb-setting-channel_mixing-promise = Computes the target taking the channel mixing into account. Off, only the CAT is considered.
tool-channelmixerrgb-setting-target_lightness-label = Lightness
tool-channelmixerrgb-setting-target_lightness-full = Target lightness
tool-channelmixerrgb-setting-target_lightness-promise = The lightness of the desired target colour after mapping.
tool-channelmixerrgb-setting-target_hue-label = Hue
tool-channelmixerrgb-setting-target_hue-full = Target hue
tool-channelmixerrgb-setting-target_hue-promise = The hue of the desired target colour after mapping.
tool-channelmixerrgb-setting-target_chroma-label = Chroma
tool-channelmixerrgb-setting-target_chroma-full = Target chroma
tool-channelmixerrgb-setting-target_chroma-promise = The chroma of the desired target colour after mapping.

tool-channelmixerrgb-tab-red = R
tool-channelmixerrgb-tab-green = G
tool-channelmixerrgb-tab-blue = B
tool-channelmixerrgb-tab-colourfulness = Colourfulness
tool-channelmixerrgb-tab-brightness = Brightness
tool-channelmixerrgb-tab-grey = Grey
tool-channelmixerrgb-control-red_r-label = Input R
tool-channelmixerrgb-control-red_r-full = Red, input R
tool-channelmixerrgb-control-red_r-promise = How much of the input's R the output's R takes. 1 = untouched.
tool-channelmixerrgb-control-red_g-label = Input G
tool-channelmixerrgb-control-red_g-full = Red, input G
tool-channelmixerrgb-control-red_g-promise = How much of the input's G the output's R takes. 0 = untouched.
tool-channelmixerrgb-control-red_b-label = Input B
tool-channelmixerrgb-control-red_b-full = Red, input B
tool-channelmixerrgb-control-red_b-promise = How much of the input's B the output's R takes. 0 = untouched.
tool-channelmixerrgb-control-red_normalise-label = Normalise channels
tool-channelmixerrgb-control-red_normalise-full = Red, normalise channels
tool-channelmixerrgb-control-red_normalise-promise = Scales the output's R so its three inputs add up to 1.
tool-channelmixerrgb-control-green_r-label = Input R
tool-channelmixerrgb-control-green_r-full = Green, input R
tool-channelmixerrgb-control-green_r-promise = How much of the input's R the output's G takes. 0 = untouched.
tool-channelmixerrgb-control-green_g-label = Input G
tool-channelmixerrgb-control-green_g-full = Green, input G
tool-channelmixerrgb-control-green_g-promise = How much of the input's G the output's G takes. 1 = untouched.
tool-channelmixerrgb-control-green_b-label = Input B
tool-channelmixerrgb-control-green_b-full = Green, input B
tool-channelmixerrgb-control-green_b-promise = How much of the input's B the output's G takes. 0 = untouched.
tool-channelmixerrgb-control-green_normalise-label = Normalise channels
tool-channelmixerrgb-control-green_normalise-full = Green, normalise channels
tool-channelmixerrgb-control-green_normalise-promise = Scales the output's G so its three inputs add up to 1.
tool-channelmixerrgb-control-blue_r-label = Input R
tool-channelmixerrgb-control-blue_r-full = Blue, input R
tool-channelmixerrgb-control-blue_r-promise = How much of the input's R the output's B takes. 0 = untouched.
tool-channelmixerrgb-control-blue_g-label = Input G
tool-channelmixerrgb-control-blue_g-full = Blue, input G
tool-channelmixerrgb-control-blue_g-promise = How much of the input's G the output's B takes. 0 = untouched.
tool-channelmixerrgb-control-blue_b-label = Input B
tool-channelmixerrgb-control-blue_b-full = Blue, input B
tool-channelmixerrgb-control-blue_b-promise = How much of the input's B the output's B takes. 1 = untouched.
tool-channelmixerrgb-control-blue_normalise-label = Normalise channels
tool-channelmixerrgb-control-blue_normalise-full = Blue, normalise channels
tool-channelmixerrgb-control-blue_normalise-promise = Scales the output's B so its three inputs add up to 1.
tool-channelmixerrgb-control-colourfulness_r-label = Input R
tool-channelmixerrgb-control-colourfulness_r-full = Colourfulness, input R
tool-channelmixerrgb-control-colourfulness_r-promise = Makes colours more or less colourful by their R, keeping their brightness. 0 = untouched.
tool-channelmixerrgb-control-colourfulness_g-label = Input G
tool-channelmixerrgb-control-colourfulness_g-full = Colourfulness, input G
tool-channelmixerrgb-control-colourfulness_g-promise = Makes colours more or less colourful by their G, keeping their brightness. 0 = untouched.
tool-channelmixerrgb-control-colourfulness_b-label = Input B
tool-channelmixerrgb-control-colourfulness_b-full = Colourfulness, input B
tool-channelmixerrgb-control-colourfulness_b-promise = Makes colours more or less colourful by their B, keeping their brightness. 0 = untouched.
tool-channelmixerrgb-control-colourfulness_normalise-label = Normalise channels
tool-channelmixerrgb-control-colourfulness_normalise-full = Colourfulness, normalise channels
tool-channelmixerrgb-control-colourfulness_normalise-promise = Takes the three weights' mean from each, so the average colour keeps its colourfulness.
tool-channelmixerrgb-control-brightness_r-label = Input R
tool-channelmixerrgb-control-brightness_r-full = Brightness, input R
tool-channelmixerrgb-control-brightness_r-promise = Brightens or darkens each pixel by its R. 0 = untouched.
tool-channelmixerrgb-control-brightness_g-label = Input G
tool-channelmixerrgb-control-brightness_g-full = Brightness, input G
tool-channelmixerrgb-control-brightness_g-promise = Brightens or darkens each pixel by its G. 0 = untouched.
tool-channelmixerrgb-control-brightness_b-label = Input B
tool-channelmixerrgb-control-brightness_b-full = Brightness, input B
tool-channelmixerrgb-control-brightness_b-promise = Brightens or darkens each pixel by its B. 0 = untouched.
tool-channelmixerrgb-control-brightness_normalise-label = Normalise channels
tool-channelmixerrgb-control-brightness_normalise-full = Brightness, normalise channels
tool-channelmixerrgb-control-brightness_normalise-promise = Takes the three weights' mean from each, so the average colour keeps its brightness.
tool-channelmixerrgb-control-grey_r-label = Input R
tool-channelmixerrgb-control-grey_r-full = Grey, input R
tool-channelmixerrgb-control-grey_r-promise = How much of the input's R the grey takes. Any weight off 0 makes the photo grey. 0 = untouched.
tool-channelmixerrgb-control-grey_g-label = Input G
tool-channelmixerrgb-control-grey_g-full = Grey, input G
tool-channelmixerrgb-control-grey_g-promise = How much of the input's G the grey takes. Any weight off 0 makes the photo grey. 0 = untouched.
tool-channelmixerrgb-control-grey_b-label = Input B
tool-channelmixerrgb-control-grey_b-full = Grey, input B
tool-channelmixerrgb-control-grey_b-promise = How much of the input's B the grey takes. Any weight off 0 makes the photo grey. 0 = untouched.
tool-channelmixerrgb-control-grey_normalise-label = Normalise channels
tool-channelmixerrgb-control-grey_normalise-full = Grey, normalise channels
tool-channelmixerrgb-control-grey_normalise-promise = Scales the grey's three weights so they add up to 1, keeping the photo's brightness.
# The colour checker's settings, darktable's own and never in an edit, drawn disabled until
# the colour checker is built.
tool-channelmixerrgb-collapsible-colour_checker = Calibrate with a colour checker
tool-channelmixerrgb-setting-chart-label = Chart
tool-channelmixerrgb-setting-chart-full = Colour checker chart
tool-channelmixerrgb-setting-chart-promise = The vendor and the type of your chart.
tool-channelmixerrgb-setting-chart-option-xrite_24_2000 = Xrite ColorChecker 24 pre-2014
tool-channelmixerrgb-setting-chart-option-xrite_24_2014 = Xrite/Calibrite ColorChecker 24 post-2014
tool-channelmixerrgb-setting-chart-option-spyder_24 = Datacolor SpyderCheckr 24 pre-2018
tool-channelmixerrgb-setting-chart-option-spyder_24_v2 = Datacolor SpyderCheckr 24 post-2018
tool-channelmixerrgb-setting-chart-option-spyder_48 = Datacolor SpyderCheckr 48 pre-2018
tool-channelmixerrgb-setting-chart-option-spyder_48_v2 = Datacolor SpyderCheckr 48 post-2018
tool-channelmixerrgb-setting-chart-option-spyder_photo = Datacolor SpyderCheckr Photo
tool-channelmixerrgb-setting-optimise_for-label = Optimise for
tool-channelmixerrgb-setting-optimise_for-full = Colours the colour checker favours
tool-channelmixerrgb-setting-optimise_for-promise = The colours the fit gets closest. Neutral colours give the lowest average ΔE, saturated colours the lowest maximum.
tool-channelmixerrgb-setting-optimise_for-option-none = None
tool-channelmixerrgb-setting-optimise_for-option-neutral = Neutral colours
tool-channelmixerrgb-setting-optimise_for-option-saturated = Saturated colours
tool-channelmixerrgb-setting-optimise_for-option-skin = Skin and soil colours
tool-channelmixerrgb-setting-optimise_for-option-foliage = Foliage colours
tool-channelmixerrgb-setting-optimise_for-option-sky = Sky and water colours
tool-channelmixerrgb-setting-optimise_for-option-average_delta_e = Average ΔE
tool-channelmixerrgb-setting-optimise_for-option-maximum_delta_e = Maximum ΔE
tool-channelmixerrgb-setting-patch_scale-label = Patch scale
tool-channelmixerrgb-setting-patch_scale-full = Colour checker patch scale
tool-channelmixerrgb-setting-patch_scale-promise = How much of each patch is read, from its centre: less where the chart's frame is loose or casts shadows.

## Tone equalizer (docs/tools/tone-equalizer.md): darktable's tone equalizer, its labels
## and tooltips in British English. A band's full name adds darktable's word for its tones;
## darktable gives a band no tooltip, so its promise says which tones it corrects.

tool-toneequal-title = Tone equalizer
    .search = tone equalizer|tone curve|tone mapping|relight|background light|shadows highlights
tool-toneequal-description = Relights the scene: brightens or darkens areas by how bright they are, as if you had lit it differently. Reach for it to open shadows or hold back a bright sky.
tool-toneequal-tab-simple = Simple
tool-toneequal-tab-advanced = Advanced
tool-toneequal-tab-masking = Masking
tool-toneequal-control-noise-label = −8 EV
tool-toneequal-control-noise-full = −8 EV, blacks
tool-toneequal-control-noise-promise = Brightens or darkens the blacks, the tones at −8 EV in the mask.
tool-toneequal-control-ultra_deep_blacks-label = −7 EV
tool-toneequal-control-ultra_deep_blacks-full = −7 EV, deep shadows
tool-toneequal-control-ultra_deep_blacks-promise = Brightens or darkens the deep shadows, the tones at −7 EV in the mask.
tool-toneequal-control-deep_blacks-label = −6 EV
tool-toneequal-control-deep_blacks-full = −6 EV, shadows
tool-toneequal-control-deep_blacks-promise = Brightens or darkens the shadows, the tones at −6 EV in the mask.
tool-toneequal-control-blacks-label = −5 EV
tool-toneequal-control-blacks-full = −5 EV, light shadows
tool-toneequal-control-blacks-promise = Brightens or darkens the light shadows, the tones at −5 EV in the mask.
tool-toneequal-control-shadows-label = −4 EV
tool-toneequal-control-shadows-full = −4 EV, mid-tones
tool-toneequal-control-shadows-promise = Brightens or darkens the mid-tones, the tones at −4 EV in the mask.
tool-toneequal-control-midtones-label = −3 EV
tool-toneequal-control-midtones-full = −3 EV, dark highlights
tool-toneequal-control-midtones-promise = Brightens or darkens the dark highlights, the tones at −3 EV in the mask.
tool-toneequal-control-highlights-label = −2 EV
tool-toneequal-control-highlights-full = −2 EV, highlights
tool-toneequal-control-highlights-promise = Brightens or darkens the highlights, the tones at −2 EV in the mask.
tool-toneequal-control-whites-label = −1 EV
tool-toneequal-control-whites-full = −1 EV, whites
tool-toneequal-control-whites-promise = Brightens or darkens the whites, the tones at −1 EV in the mask.
tool-toneequal-control-speculars-label = +0 EV
tool-toneequal-control-speculars-full = +0 EV, speculars
tool-toneequal-control-speculars-promise = Brightens or darkens the speculars, the tones at 0 EV in the mask and above.
tool-toneequal-control-smoothing-label = Curve smoothing
tool-toneequal-control-smoothing-full = Curve smoothing
tool-toneequal-control-smoothing-promise = Positive values give more progressive tone transitions, but the curve might oscillate in some settings. Negative values avoid oscillations and behave more robustly, but may give abrupt tone transitions and damage local contrast.
tool-toneequal-control-exposure_boost-label = Mask exposure compensation
tool-toneequal-control-exposure_boost-full = Mask exposure compensation
tool-toneequal-control-exposure_boost-promise = Slides the mask's average exposure along the bands, for better control of the exposure correction with the bands there are.
tool-toneequal-control-contrast_boost-label = Mask contrast compensation
tool-toneequal-control-contrast_boost-full = Mask contrast compensation
tool-toneequal-control-contrast_boost-promise = Counters the guided filter's averaging and dilates the mask's contrast around −4 EV, spreading its exposure histogram over more bands for better control of the exposure correction. Acts with the guided filter and EIGF only.
tool-toneequal-control-method-label = Luminance estimator
tool-toneequal-control-method-full = Luminance estimator
tool-toneequal-control-method-promise = Preview the mask and choose the estimator that gives the higher contrast between areas to dodge and areas to burn.
tool-toneequal-control-method-option-rgb_average = RGB average
tool-toneequal-control-method-option-hsl_lightness = HSL lightness
tool-toneequal-control-method-option-hsv_value = HSV value / RGB max
tool-toneequal-control-method-option-rgb_sum = RGB sum
tool-toneequal-control-method-option-rgb_euclidean_norm = RGB euclidean norm
tool-toneequal-control-method-option-rgb_power_norm = RGB power norm
tool-toneequal-control-method-option-rgb_geometric_mean = RGB geometric mean
tool-toneequal-control-details-label = Preserve details
tool-toneequal-control-details-full = Preserve details
tool-toneequal-control-details-promise = No affects global and local contrast, safe if you only add contrast. The guided filter affects only global contrast and tries to preserve local contrast; the averaged guided filter is a geometric mean of no and the guided filter. EIGF, the exposure-independent guided filter, smooths shadows and highlights the same way, where the guided filter smooths the highlights less; averaged EIGF is a geometric mean of no and EIGF.
tool-toneequal-control-details-option-none = No
tool-toneequal-control-details-option-averaged_guided_filter = Averaged guided filter
tool-toneequal-control-details-option-guided_filter = Guided filter
tool-toneequal-control-details-option-averaged_eigf = Averaged EIGF
tool-toneequal-control-details-option-eigf = EIGF
tool-toneequal-control-iterations-label = Filter diffusion
tool-toneequal-control-iterations-full = Filter diffusion
tool-toneequal-control-iterations-promise = How many times the filter runs: more passes diffuse its edges, at the expense of speed.
tool-toneequal-control-blending-label = Smoothing diameter
tool-toneequal-control-blending-full = Smoothing diameter
tool-toneequal-control-blending-promise = Diameter of the blur in percent of the image's largest side.
tool-toneequal-control-feathering-label = Edges refinement/feathering
tool-toneequal-control-feathering-full = Edges refinement/feathering
tool-toneequal-control-feathering-promise = Precision of the feathering: higher values make the mask follow edges more closely but may void the smoothing; lower values give smoother gradients and better smoothing but may lead to inaccurate edges and halos.
tool-toneequal-control-quantization-label = Mask quantisation
tool-toneequal-control-quantization-full = Mask quantisation
tool-toneequal-control-quantization-promise = 0 turns quantisation off. Higher values posterise the luminance mask, helping the filter produce piecewise smooth areas with high feathering values.
# darktable's display of the mask, a view mode no edit stores, drawn disabled until the
# mask preview comes.
tool-toneequal-setting-display_exposure_mask-label = Display exposure mask
tool-toneequal-setting-display_exposure_mask-full = Display exposure mask
tool-toneequal-setting-display_exposure_mask-promise = Shows the mask the bands act on in place of the photo.

## Colour balance RGB (docs/tools/colour-balance-rgb.md): darktable's color balance rgb,
## its labels and tooltips in British English. A full name says which group a short label
## is in, as darktable's shortcuts do.

tool-colorbalancergb-title = Colour balance RGB
    .search = color balance rgb|offset power slope|cdl|color grading|contrast|chroma_highlights|hue|vibrance|saturation
tool-colorbalancergb-description = Colour grading: shifts the colour of the shadows, mid-tones and highlights separately, each picked out by a soft mask. Reach for it to give a photo its colour mood.
tool-colorbalancergb-tab-master = Master
tool-colorbalancergb-tab-four_ways = 4 ways
tool-colorbalancergb-tab-masks = Masks
tool-colorbalancergb-group-linear_chroma_grading = Linear chroma grading
tool-colorbalancergb-group-perceptual_saturation_grading = Perceptual saturation grading
tool-colorbalancergb-group-perceptual_brilliance_grading = Perceptual brilliance grading
tool-colorbalancergb-group-global_offset = Global offset
tool-colorbalancergb-group-shadows_lift = Shadows lift
tool-colorbalancergb-group-highlights_gain = Highlights gain
tool-colorbalancergb-group-power = Power
tool-colorbalancergb-group-luminance_ranges = Luminance ranges
tool-colorbalancergb-group-threshold = Threshold
tool-colorbalancergb-control-hue_angle-label = Hue shift
tool-colorbalancergb-control-hue_angle-full = Hue shift
tool-colorbalancergb-control-hue_angle-promise = Rotates all hues by an angle, at the same luminance.
tool-colorbalancergb-control-vibrance-label = Global vibrance
tool-colorbalancergb-control-vibrance-full = Global vibrance
tool-colorbalancergb-control-vibrance-promise = Increases colourfulness mostly on low-chroma colours.
tool-colorbalancergb-control-contrast-label = Contrast
tool-colorbalancergb-control-contrast-full = Contrast
tool-colorbalancergb-control-contrast-promise = Increases the contrast at constant chromaticity.
tool-colorbalancergb-control-chroma_global-label = Global chroma
tool-colorbalancergb-control-chroma_global-full = Global chroma
tool-colorbalancergb-control-chroma_global-promise = Increases colourfulness at the same luminance, globally.
tool-colorbalancergb-control-chroma_shadows-label = Shadows
tool-colorbalancergb-control-chroma_shadows-full = Shadows chroma
tool-colorbalancergb-control-chroma_shadows-promise = Increases colourfulness at the same luminance, mostly in the shadows.
tool-colorbalancergb-control-chroma_midtones-label = Mid-tones
tool-colorbalancergb-control-chroma_midtones-full = Mid-tones chroma
tool-colorbalancergb-control-chroma_midtones-promise = Increases colourfulness at the same luminance, mostly in the mid-tones.
tool-colorbalancergb-control-chroma_highlights-label = Highlights
tool-colorbalancergb-control-chroma_highlights-full = Highlights chroma
tool-colorbalancergb-control-chroma_highlights-promise = Increases colourfulness at the same luminance, mostly in the highlights.
tool-colorbalancergb-control-saturation_global-label = Global saturation
tool-colorbalancergb-control-saturation_global-full = Global saturation
tool-colorbalancergb-control-saturation_global-promise = Adds or removes saturation by an absolute amount.
tool-colorbalancergb-control-saturation_shadows-label = Shadows
tool-colorbalancergb-control-saturation_shadows-full = Shadows saturation
tool-colorbalancergb-control-saturation_shadows-promise = Increases or decreases saturation in proportion to each pixel's own, mostly in the shadows.
tool-colorbalancergb-control-saturation_midtones-label = Mid-tones
tool-colorbalancergb-control-saturation_midtones-full = Mid-tones saturation
tool-colorbalancergb-control-saturation_midtones-promise = Increases or decreases saturation in proportion to each pixel's own, mostly in the mid-tones.
tool-colorbalancergb-control-saturation_highlights-label = Highlights
tool-colorbalancergb-control-saturation_highlights-full = Highlights saturation
tool-colorbalancergb-control-saturation_highlights-promise = Increases or decreases saturation in proportion to each pixel's own, mostly in the highlights.
tool-colorbalancergb-control-brilliance_global-label = Global brilliance
tool-colorbalancergb-control-brilliance_global-full = Global brilliance
tool-colorbalancergb-control-brilliance_global-promise = Adds or removes brilliance by an absolute amount.
tool-colorbalancergb-control-brilliance_shadows-label = Shadows
tool-colorbalancergb-control-brilliance_shadows-full = Shadows brilliance
tool-colorbalancergb-control-brilliance_shadows-promise = Increases or decreases brilliance in proportion to each pixel's own, mostly in the shadows.
tool-colorbalancergb-control-brilliance_midtones-label = Mid-tones
tool-colorbalancergb-control-brilliance_midtones-full = Mid-tones brilliance
tool-colorbalancergb-control-brilliance_midtones-promise = Increases or decreases brilliance in proportion to each pixel's own, mostly in the mid-tones.
tool-colorbalancergb-control-brilliance_highlights-label = Highlights
tool-colorbalancergb-control-brilliance_highlights-full = Highlights brilliance
tool-colorbalancergb-control-brilliance_highlights-promise = Increases or decreases brilliance in proportion to each pixel's own, mostly in the highlights.
tool-colorbalancergb-control-global_y-label = Luminance
tool-colorbalancergb-control-global_y-full = Global offset luminance
tool-colorbalancergb-control-global_y-promise = Global luminance offset.
tool-colorbalancergb-control-global_h-label = Hue
tool-colorbalancergb-control-global_h-full = Global offset hue
tool-colorbalancergb-control-global_h-promise = Hue of the global colour offset.
tool-colorbalancergb-control-global_c-label = Chroma
tool-colorbalancergb-control-global_c-full = Global offset chroma
tool-colorbalancergb-control-global_c-promise = Chroma of the global colour offset.
tool-colorbalancergb-control-shadows_y-label = Luminance
tool-colorbalancergb-control-shadows_y-full = Shadows lift luminance
tool-colorbalancergb-control-shadows_y-promise = Luminance gain in the shadows.
tool-colorbalancergb-control-shadows_h-label = Hue
tool-colorbalancergb-control-shadows_h-full = Shadows lift hue
tool-colorbalancergb-control-shadows_h-promise = Hue of the colour gain in the shadows.
tool-colorbalancergb-control-shadows_c-label = Chroma
tool-colorbalancergb-control-shadows_c-full = Shadows lift chroma
tool-colorbalancergb-control-shadows_c-promise = Chroma of the colour gain in the shadows.
tool-colorbalancergb-control-highlights_y-label = Luminance
tool-colorbalancergb-control-highlights_y-full = Highlights gain luminance
tool-colorbalancergb-control-highlights_y-promise = Luminance gain in the highlights.
tool-colorbalancergb-control-highlights_h-label = Hue
tool-colorbalancergb-control-highlights_h-full = Highlights gain hue
tool-colorbalancergb-control-highlights_h-promise = Hue of the colour gain in the highlights.
tool-colorbalancergb-control-highlights_c-label = Chroma
tool-colorbalancergb-control-highlights_c-full = Highlights gain chroma
tool-colorbalancergb-control-highlights_c-promise = Chroma of the colour gain in the highlights.
tool-colorbalancergb-control-midtones_y-label = Luminance
tool-colorbalancergb-control-midtones_y-full = Power luminance
tool-colorbalancergb-control-midtones_y-promise = Luminance exponent in the mid-tones.
tool-colorbalancergb-control-midtones_h-label = Hue
tool-colorbalancergb-control-midtones_h-full = Power hue
tool-colorbalancergb-control-midtones_h-promise = Hue of the colour exponent in the mid-tones.
tool-colorbalancergb-control-midtones_c-label = Chroma
tool-colorbalancergb-control-midtones_c-full = Power chroma
tool-colorbalancergb-control-midtones_c-promise = Chroma of the colour exponent in the mid-tones.
tool-colorbalancergb-control-saturation_formula-label = Saturation formula
tool-colorbalancergb-control-saturation_formula-full = Saturation formula
tool-colorbalancergb-control-saturation_formula-promise = Chooses the uniform colour space saturation is computed in.
tool-colorbalancergb-control-saturation_formula-option-jzazbz = JzAzBz (2021)
tool-colorbalancergb-control-saturation_formula-option-ucs = UCS (2022)
tool-colorbalancergb-control-shadows_weight-label = Shadows fall-off
tool-colorbalancergb-control-shadows_weight-full = Shadows fall-off
tool-colorbalancergb-control-shadows_weight-promise = Weight of the shadows over the whole tonal range.
tool-colorbalancergb-control-mask_grey_fulcrum-label = Mask middle-grey fulcrum
tool-colorbalancergb-control-mask_grey_fulcrum-full = Mask middle-grey fulcrum
tool-colorbalancergb-control-mask_grey_fulcrum-promise = Position of the middle-grey reference for masking.
tool-colorbalancergb-control-highlights_weight-label = Highlights fall-off
tool-colorbalancergb-control-highlights_weight-full = Highlights fall-off
tool-colorbalancergb-control-highlights_weight-promise = Weight of the highlights over the whole tonal range.
tool-colorbalancergb-control-white_fulcrum-label = White fulcrum
tool-colorbalancergb-control-white_fulcrum-full = White fulcrum
tool-colorbalancergb-control-white_fulcrum-promise = Peak white luminance used to normalise the power function.
tool-colorbalancergb-control-grey_fulcrum-label = Contrast grey fulcrum
tool-colorbalancergb-control-grey_fulcrum-full = Contrast grey fulcrum
tool-colorbalancergb-control-grey_fulcrum-promise = Peak grey luminance used to normalise the power function: the grey Contrast keeps.
# The pickers on the four hues' quads and the two fulcrums': darktable's quad tooltips,
# its "pick color from image" where it sets none.
tool-colorbalancergb-picker-global_hue-full = Global offset hue: pick from the photo
tool-colorbalancergb-picker-global_hue-promise = Picks the colour opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-shadows_hue-full = Shadows lift hue: pick from the photo
tool-colorbalancergb-picker-shadows_hue-promise = Picks the colour opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-highlights_hue-full = Highlights gain hue: pick from the photo
tool-colorbalancergb-picker-highlights_hue-promise = Picks the colour opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-midtones_hue-full = Power hue: pick from the photo
tool-colorbalancergb-picker-midtones_hue-promise = Picks the colour opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-white_fulcrum-full = White fulcrum: pick from the photo
tool-colorbalancergb-picker-white_fulcrum-promise = Pick colour from the photo
tool-colorbalancergb-picker-grey_fulcrum-full = Contrast grey fulcrum: pick from the photo
tool-colorbalancergb-picker-grey_fulcrum-promise = Pick colour from the photo
# The four hue pickers' cursors on the Toolbar (decision 104), each named for its picker;
# the fulcrums' pickers have none.
tool-colorbalancergb-cursor-global_hue = Global offset hue picker
tool-colorbalancergb-cursor-global_hue-how = Click an area, or drag one
tool-colorbalancergb-cursor-shadows_hue = Shadows lift hue picker
tool-colorbalancergb-cursor-shadows_hue-how = Click an area, or drag one
tool-colorbalancergb-cursor-highlights_hue = Highlights gain hue picker
tool-colorbalancergb-cursor-highlights_hue-how = Click an area, or drag one
tool-colorbalancergb-cursor-midtones_hue = Power hue picker
tool-colorbalancergb-cursor-midtones_hue-how = Click an area, or drag one

## Tone curve (docs/tools/tone-curve.md)

tool-tone_curve-title = Tone curve
    .search = rgb curve
tool-tone_curve-description = Reshapes the photo’s tones with a curve, brightening or darkening any range of them. Reach for it to fine-tune contrast.
tool-tone_curve-tab-luma = Luma
tool-tone_curve-tab-rgb = RGB
tool-tone_curve-tab-red = Red
tool-tone_curve-tab-green = Green
tool-tone_curve-tab-blue = Blue
tool-tone_curve-tab-advanced = Advanced
tool-tone_curve-control-luma_points-label = Luma
# The curve's name for screen readers.
tool-tone_curve-control-luma_points-full = Tone curve, Luma
tool-tone_curve-control-luma_points-promise = Moves each tone to the height you draw it at. Keeps colours. A straight line = untouched.
tool-tone_curve-control-preserve_colour-label = Preserve colours
tool-tone_curve-control-preserve_colour-full = Preserve colours in the Luma curve
tool-tone_curve-control-preserve_colour-promise = What the Luma curve acts on in each pixel, scaling its three channels alike so its colour stays.
tool-tone_curve-control-preserve_colour-option-luminance = Luminance
tool-tone_curve-control-preserve_colour-option-max_rgb = Max RGB
tool-tone_curve-control-preserve_colour-option-average_rgb = Average RGB
tool-tone_curve-control-preserve_colour-option-sum_rgb = Sum RGB
tool-tone_curve-control-preserve_colour-option-norm_rgb = Norm RGB
tool-tone_curve-control-preserve_colour-option-basic_power = Basic power

## Haze removal (docs/tools/haze-removal.md): darktable's haze removal, its labels and
## tooltips in British English.

tool-hazeremoval-title = Haze removal
    .search = haze removal|dehaze|defog|smoke|smog
tool-hazeremoval-description = Removes haze and fog, bringing back contrast and colour in distant parts of the scene. Reach for it on landscapes that look washed out.
tool-hazeremoval-control-strength-label = Strength
tool-hazeremoval-control-strength-full = Strength
tool-hazeremoval-control-strength-promise = Amount of haze reduction.
tool-hazeremoval-control-distance-label = Distance
tool-hazeremoval-control-distance-full = Distance
tool-hazeremoval-control-distance-promise = Limit haze removal up to a specific spatial depth.

## RGB primaries (docs/tools/rgb-primaries.md): darktable's rgb primaries, its labels
## and tooltips in British English.

tool-primaries-title = RGB primaries
    .search = rgb primaries
tool-primaries-description = Moves the red, green and blue primaries, shifting every colour made of them. Reach for it for broad, smooth colour grading.
tool-primaries-control-red_hue-label = Red hue
tool-primaries-control-red_hue-full = Red hue
tool-primaries-control-red_hue-promise = Shifts red towards yellow or magenta.
tool-primaries-control-red_purity-label = Red purity
tool-primaries-control-red_purity-full = Red purity
tool-primaries-control-red_purity-promise = Red primary purity.
tool-primaries-control-green_hue-label = Green hue
tool-primaries-control-green_hue-full = Green hue
tool-primaries-control-green_hue-promise = Shifts green towards cyan or yellow.
tool-primaries-control-green_purity-label = Green purity
tool-primaries-control-green_purity-full = Green purity
tool-primaries-control-green_purity-promise = Green primary purity.
tool-primaries-control-blue_hue-label = Blue hue
tool-primaries-control-blue_hue-full = Blue hue
tool-primaries-control-blue_hue-promise = Shifts blue towards magenta or cyan.
tool-primaries-control-blue_purity-label = Blue purity
tool-primaries-control-blue_purity-full = Blue purity
tool-primaries-control-blue_purity-promise = Blue primary purity.
tool-primaries-control-achromatic_tint_hue-label = Tint hue
tool-primaries-control-achromatic_tint_hue-full = Tint hue
tool-primaries-control-achromatic_tint_hue-promise = Tint hue.
tool-primaries-control-achromatic_tint_purity-label = Tint purity
tool-primaries-control-achromatic_tint_purity-full = Tint purity
tool-primaries-control-achromatic_tint_purity-promise = Tint purity.

## Grain (docs/tools/grain.md): darktable's grain, its labels and tooltips in British
## English.

tool-grain-title = Grain
    .search = grain
tool-grain-description = Adds grain like that of film, fine or coarse. Reach for it to give a photo the texture of film.
tool-grain-control-scale-label = Coarseness
tool-grain-control-scale-full = Coarseness
tool-grain-control-scale-promise = The grain size (~ISO of the film).
tool-grain-control-strength-label = Strength
tool-grain-control-strength-full = Strength
tool-grain-control-strength-promise = The strength of applied grain.
tool-grain-control-midtones_bias-label = Mid-tones bias
tool-grain-control-midtones_bias-full = Mid-tones bias
tool-grain-control-midtones_bias-promise = Amount of mid-tones bias from the photographic paper response modelling. The greater the bias, the more pronounced the fall-off of the grain in shadows and highlights.

## Soften (docs/tools/soften.md): darktable's soften, its labels and tooltips in
## British English.

tool-soften-title = Soften
    .search = soften
tool-soften-description = Adds a soft glow by blending a blurred, brighter copy over the photo. Reach for it for a dreamy look.
tool-soften-control-size-label = Size
tool-soften-control-size-full = Size
tool-soften-control-size-promise = The size of blur.
tool-soften-control-saturation-label = Saturation
tool-soften-control-saturation-full = Saturation
tool-soften-control-saturation-promise = The saturation of blur.
tool-soften-control-brightness-label = Brightness
tool-soften-control-brightness-full = Brightness
tool-soften-control-brightness-promise = The brightness of blur.
tool-soften-control-amount-label = Mix
tool-soften-control-amount-full = Mix
tool-soften-control-amount-promise = The mix of effect.

## Vignetting (docs/tools/vignetting.md): darktable's vignetting, its labels and tooltips
## in British English.

tool-vignette-title = Vignetting
    .search = vignetting
tool-vignette-description = Darkens or lightens the photo towards its edges, as a lens does. Reach for it to draw the eye to the centre.
tool-vignette-control-scale-label = Fall-off start
tool-vignette-control-scale-full = Fall-off start
tool-vignette-control-scale-promise = The radii scale of vignette for start of fall-off.
tool-vignette-control-falloff_scale-label = Fall-off radius
tool-vignette-control-falloff_scale-full = Fall-off radius
tool-vignette-control-falloff_scale-promise = The radii scale of vignette for end of fall-off.
tool-vignette-control-brightness-label = Brightness
tool-vignette-control-brightness-full = Brightness
tool-vignette-control-brightness-promise = Strength of effect on brightness.
tool-vignette-control-saturation-label = Saturation
tool-vignette-control-saturation-full = Saturation
tool-vignette-control-saturation-promise = Strength of effect on saturation.
tool-vignette-group-position_form = Position / form
tool-vignette-control-center_x-label = Horizontal centre
tool-vignette-control-center_x-full = Horizontal centre
tool-vignette-control-center_x-promise = Horizontal offset of centre of the effect.
tool-vignette-control-center_y-label = Vertical centre
tool-vignette-control-center_y-full = Vertical centre
tool-vignette-control-center_y-promise = Vertical offset of centre of the effect.
tool-vignette-control-shape-label = Shape
tool-vignette-control-shape-full = Shape
tool-vignette-control-shape-promise = Shape factor: 0 produces a rectangle, 1 a circle or ellipse, 2 a diamond.
tool-vignette-control-autoratio-label = Automatic ratio
tool-vignette-control-autoratio-full = Automatic ratio
tool-vignette-control-autoratio-promise = Enable to have the ratio automatically follow the image size.
tool-vignette-control-whratio-label = Width/height ratio
tool-vignette-control-whratio-full = Width/height ratio
tool-vignette-control-whratio-promise = Width-to-height ratio.
tool-vignette-control-dithering-label = Dithering
tool-vignette-control-dithering-full = Dithering
tool-vignette-control-dithering-promise = Add some level of random noise to prevent banding.
tool-vignette-control-dithering-option-off = Off
tool-vignette-control-dithering-option-output_8bit = 8-bit output
tool-vignette-control-dithering-option-output_16bit = 16-bit output

## What each utility is for, and the words of a header's tooltip (docs/gui.md, The
## header's tooltip): a utility's sentence, tool-<utility>-description; what a tool is
## for and what it works on, from darktable's purpose, input and output, tool-kind-<kind>
## and tool-works-<place>; and darktable's input, process and output lines word for
## word, tool-fact-<darktable's words>, each one of darktable's own strings.

tool-scopes-description = Shows how the photo’s tones and colours are spread: histogram, waveform, RGB parade or vectorscope. Reach for it to judge exposure and colour as you edit.
tool-history-description = Lists every change to the photo’s edit, newest first. Reach for it to go back to an earlier step.
tool-metadata-description = What the camera recorded about the selected photos, and the title, description, creator, rights and place you write about them.
tool-info-description = The active photo’s name, date, camera and exposure at a glance.
tool-styles-description = Lists the styles applied to the photo, then every style you have made. Reach for it to give a photo a look in one click.

tool-kind-corrective = Fixes a problem
tool-kind-creative = Creates a look
tool-kind-corrective-and-creative = Fixes a problem and shapes the look
tool-kind-corrective-or-creative = Fixes a problem or creates a look
tool-kind-mandatory = Needed by every photo
tool-kind-utility = Edits nothing of its own

tool-works-raw = The camera’s raw data
tool-works-scene = The light as captured
tool-works-scene-to-display = From the light as captured to the finished picture
tool-works-display = The finished picture

tool-fact-linear-raw-scene-referred = linear, raw, scene-referred
tool-fact-linear-raw = linear, raw
tool-fact-reconstruction-raw = reconstruction, raw
tool-fact-linear-rgb-scene-referred = linear, RGB, scene-referred
tool-fact-linear-rgb-display-referred = linear, RGB, display-referred
tool-fact-linear-rgb = linear, RGB
tool-fact-linear-rgb-or-xyz = linear, RGB or XYZ
tool-fact-quasi-linear-rgb = quasi-linear, RGB
tool-fact-quasi-linear-rgb-scene-referred = quasi-linear, RGB, scene-referred
tool-fact-non-linear-rgb = non-linear, RGB
tool-fact-non-linear-rgb-scene-referred = non-linear, RGB, scene-referred
tool-fact-non-linear-rgb-display-referred = non-linear, RGB, display-referred
tool-fact-non-linear-lab = non-linear, Lab
tool-fact-non-linear-lab-display-referred = non-linear, Lab, display-referred
tool-fact-linear-or-non-linear-lab-display-referred = linear or non-linear, Lab, display-referred
tool-fact-linear-or-non-linear-rgb-scene-referred = linear or non-linear, RGB, scene-referred
tool-fact-non-linear-rgb-or-lab-display-referred = non-linear, RGB or Lab, display-referred
tool-fact-geometric-rgb = geometric, RGB
tool-fact-frequential-rgb = frequential, RGB
tool-fact-defined-by-profile = defined by profile

## The Metadata tool (decision 110): its groups, each title's tooltip naming its standard,
## ＋ and its facts, each with its label, its standard and tag, and why it has none.

tool-metadata-rows-shown = Rows Shown
tool-metadata-collapsible-basic = Basic
tool-metadata-collapsible-basic-tip = The file and the library
tool-metadata-collapsible-camera = Camera
tool-metadata-collapsible-camera-tip = EXIF · IFD0
tool-metadata-collapsible-exposure = Exposure
tool-metadata-collapsible-exposure-tip = EXIF · Exif IFD
tool-metadata-collapsible-lens = Lens
tool-metadata-collapsible-lens-tip = EXIF · Exif IFD, lens
tool-metadata-collapsible-gps = GPS
tool-metadata-collapsible-gps-tip = EXIF · GPS IFD
tool-metadata-collapsible-content = Content
tool-metadata-collapsible-content-tip = IPTC Core · Content
tool-metadata-collapsible-creator = Creator
tool-metadata-collapsible-creator-tip = IPTC Core · Contact
tool-metadata-collapsible-rights = Rights
tool-metadata-collapsible-rights-tip = IPTC Core · Status, rights
tool-metadata-collapsible-location = Location
tool-metadata-collapsible-location-tip = IPTC Core · Image, location
tool-metadata-collapsible-status = Status
tool-metadata-collapsible-status-tip = IPTC Core · Status
tool-metadata-fact-file_name-label = File name
tool-metadata-fact-file_name-tag = The file’s name
tool-metadata-fact-file_name-none = The file has no name
tool-metadata-fact-folder-label = Folder
tool-metadata-fact-folder-tag = The file’s folder
tool-metadata-fact-folder-none = The file’s folder is not known
tool-metadata-fact-type-label = Type
tool-metadata-fact-type-tag = The format its decoder names
tool-metadata-fact-type-none = The decoder names no format
tool-metadata-fact-size_on_disk-label = Size on disk
tool-metadata-fact-size_on_disk-tag = The file’s size
tool-metadata-fact-size_on_disk-none = The file’s size is not known
tool-metadata-fact-dimensions-label = Dimensions
tool-metadata-fact-dimensions-tag = The developed image, in pixels
tool-metadata-fact-dimensions-none = The image’s size is not known
tool-metadata-fact-edited-label = Edited
tool-metadata-fact-edited-tag = The last step of the edit’s history
tool-metadata-fact-edited-none = The edit’s history has not been read
tool-metadata-field-copy-label = Copy
tool-metadata-field-copy-tag = The name of this virtual copy · apertrust:name
tool-metadata-fact-preserved_file_name-label = Preserved file name
tool-metadata-fact-preserved_file_name-tag = PreservedFileName · xmpMM:PreservedFileName
tool-metadata-fact-preserved_file_name-none = No file name was preserved
tool-metadata-field-title-label = Title
tool-metadata-field-title-tag = Title, which IPTC Core files under Status · dc:title
tool-metadata-field-headline-label = Headline
tool-metadata-field-headline-tag = Headline · photoshop:Headline
tool-metadata-field-description-label = Description
tool-metadata-field-description-tag = Description · dc:description
tool-metadata-field-description_writer-label = Description writer
tool-metadata-field-description_writer-tag = Description Writer · photoshop:CaptionWriter
tool-metadata-field-notes-label = Notes
tool-metadata-field-notes-tag = Apertrust’s own · apertrust:notes
tool-metadata-field-creator-label = Creator
tool-metadata-field-creator-tag = Creator · dc:creator
tool-metadata-field-job_title-label = Job title
tool-metadata-field-job_title-tag = Creator’s Jobtitle · photoshop:AuthorsPosition
tool-metadata-field-email-label = Email
tool-metadata-field-email-tag = Creator’s Contact Info, Email · Iptc4xmpCore:CiEmailWork
tool-metadata-field-website-label = Website
tool-metadata-field-website-tag = Creator’s Contact Info, Web URL · Iptc4xmpCore:CiUrlWork
tool-metadata-field-phone-label = Phone
tool-metadata-field-phone-tag = Creator’s Contact Info, Phone · Iptc4xmpCore:CiTelWork
tool-metadata-field-address-label = Address
tool-metadata-field-address-tag = Creator’s Contact Info, Address · Iptc4xmpCore:CiAdrExtadr
tool-metadata-field-contact_city-label = City
tool-metadata-field-contact_city-tag = Creator’s Contact Info, City · Iptc4xmpCore:CiAdrCity
tool-metadata-field-contact_region-label = Region
tool-metadata-field-contact_region-tag = Creator’s Contact Info, State/Province · Iptc4xmpCore:CiAdrRegion
tool-metadata-field-postal_code-label = Postal code
tool-metadata-field-postal_code-tag = Creator’s Contact Info, Postal Code · Iptc4xmpCore:CiAdrPcode
tool-metadata-field-contact_country-label = Country
tool-metadata-field-contact_country-tag = Creator’s Contact Info, Country · Iptc4xmpCore:CiAdrCtry
tool-metadata-field-copyright_notice-label = Copyright notice
tool-metadata-field-copyright_notice-tag = Copyright Notice · dc:rights
tool-metadata-field-usage_terms-label = Rights usage terms
tool-metadata-field-usage_terms-tag = Rights Usage Terms · xmpRights:UsageTerms
tool-metadata-field-copyright_status-label = Copyright status
tool-metadata-field-copyright_status-tag = Copyright Status · xmpRights:Marked
tool-metadata-field-web_statement-label = Web statement
tool-metadata-field-web_statement-tag = Web Statement of Rights · xmpRights:WebStatement
tool-metadata-field-credit_line-label = Credit line
tool-metadata-field-credit_line-tag = Credit Line · photoshop:Credit
tool-metadata-field-source-label = Source
tool-metadata-field-source-tag = Source · photoshop:Source
tool-metadata-field-publisher-label = Publisher
tool-metadata-field-publisher-tag = Publisher, Dublin Core · dc:publisher
tool-metadata-field-sublocation-label = Sublocation
tool-metadata-field-sublocation-tag = Sublocation · Iptc4xmpCore:Location
tool-metadata-field-city-label = City
tool-metadata-field-city-tag = City · photoshop:City
tool-metadata-field-state-label = State or province
tool-metadata-field-state-tag = Province/State · photoshop:State
tool-metadata-field-country-label = Country
tool-metadata-field-country-tag = Country · photoshop:Country
tool-metadata-field-country_code-label = Country code
tool-metadata-field-country_code-tag = Country Code · Iptc4xmpCore:CountryCode
tool-metadata-field-instructions-label = Instructions
tool-metadata-field-instructions-tag = Instructions · photoshop:Instructions
tool-metadata-field-job_id-label = Job id
tool-metadata-field-job_id-tag = Job Identifier · photoshop:TransmissionReference
tool-metadata-field-date_created-label = Date created
tool-metadata-field-date_created-tag = Date Created · photoshop:DateCreated
tool-metadata-field-genre-label = Intellectual genre
tool-metadata-field-genre-tag = Intellectual Genre · Iptc4xmpCore:IntellectualGenre
tool-metadata-field-scene_code-label = Scene code
tool-metadata-field-scene_code-tag = Scene Code · Iptc4xmpCore:Scene
tool-metadata-field-subject_code-label = Subject code
tool-metadata-field-subject_code-tag = Subject Code · Iptc4xmpCore:SubjectCode
tool-metadata-fact-make-label = Make
tool-metadata-fact-make-tag = Make, EXIF 0x010F
tool-metadata-fact-make-none = The camera recorded no make
tool-metadata-fact-model-label = Model
tool-metadata-fact-model-tag = Model, EXIF 0x0110
tool-metadata-fact-model-none = The camera recorded no model
tool-metadata-fact-serial_number-label = Serial number
tool-metadata-fact-serial_number-tag = BodySerialNumber, EXIF 0xA431
tool-metadata-fact-serial_number-none = The camera recorded no serial number
tool-metadata-fact-owner_name-label = Owner name
tool-metadata-fact-owner_name-tag = CameraOwnerName, EXIF 0xA430
tool-metadata-fact-owner_name-none = The camera recorded no owner
tool-metadata-fact-software-label = Software
tool-metadata-fact-software-tag = Software, EXIF 0x0131
tool-metadata-fact-software-none = The camera recorded no software
tool-metadata-fact-date_time-label = Date and time (file)
tool-metadata-fact-date_time-tag = DateTime, SubSecTime and OffsetTime, EXIF 0x0132, 0x9290, 0x9010
tool-metadata-fact-date_time-none = The camera recorded no date and time
tool-metadata-fact-orientation-label = Orientation
tool-metadata-fact-orientation-tag = Orientation, EXIF 0x0112
tool-metadata-fact-orientation-none = The camera recorded no orientation
tool-metadata-fact-resolution-label = Resolution
tool-metadata-fact-resolution-tag = XResolution, YResolution and ResolutionUnit, EXIF 0x011A, 0x011B, 0x0128
tool-metadata-fact-resolution-none = The camera recorded no resolution
tool-metadata-fact-artist-label = Artist (camera’s)
tool-metadata-fact-artist-tag = Artist, EXIF 0x013B
tool-metadata-fact-artist-none = The camera recorded no artist
tool-metadata-fact-copyright-label = Copyright (camera’s)
tool-metadata-fact-copyright-tag = Copyright, EXIF 0x8298
tool-metadata-fact-copyright-none = The camera recorded no copyright
tool-metadata-fact-image_description-label = Image description (camera’s)
tool-metadata-fact-image_description-tag = ImageDescription, EXIF 0x010E
tool-metadata-fact-image_description-none = The camera recorded no description
tool-metadata-fact-date_taken-label = Date taken
tool-metadata-fact-date_taken-tag = DateTimeOriginal, SubSecTimeOriginal and OffsetTimeOriginal, EXIF 0x9003, 0x9291, 0x9011
tool-metadata-fact-date_taken-none = The camera recorded no date taken
tool-metadata-fact-date_digitised-label = Date digitised
tool-metadata-fact-date_digitised-tag = DateTimeDigitized, SubSecTimeDigitized and OffsetTimeDigitized, EXIF 0x9004, 0x9292, 0x9012
tool-metadata-fact-date_digitised-none = The camera recorded no date digitised
tool-metadata-fact-exposure_time-label = Exposure time
tool-metadata-fact-exposure_time-tag = ExposureTime, EXIF 0x829A
tool-metadata-fact-exposure_time-none = The camera recorded no exposure time
tool-metadata-fact-aperture-label = Aperture
tool-metadata-fact-aperture-tag = FNumber, EXIF 0x829D
tool-metadata-fact-aperture-none = The camera recorded no aperture
tool-metadata-fact-iso-label = ISO
tool-metadata-fact-iso-tag = PhotographicSensitivity and SensitivityType, EXIF 0x8827, 0x8830
tool-metadata-fact-iso-none = The camera recorded no ISO
tool-metadata-fact-exposure_bias-label = Exposure bias
tool-metadata-fact-exposure_bias-tag = ExposureBiasValue, EXIF 0x9204
tool-metadata-fact-exposure_bias-none = The camera recorded no exposure bias
tool-metadata-fact-exposure_program-label = Exposure program
tool-metadata-fact-exposure_program-tag = ExposureProgram, EXIF 0x8822
tool-metadata-fact-exposure_program-none = The camera recorded no exposure program
tool-metadata-fact-exposure_mode-label = Exposure mode
tool-metadata-fact-exposure_mode-tag = ExposureMode, EXIF 0xA402
tool-metadata-fact-exposure_mode-none = The camera recorded no exposure mode
tool-metadata-fact-metering_mode-label = Metering mode
tool-metadata-fact-metering_mode-tag = MeteringMode, EXIF 0x9207
tool-metadata-fact-metering_mode-none = The camera recorded no metering mode
tool-metadata-fact-flash-label = Flash
tool-metadata-fact-flash-tag = Flash, EXIF 0x9209
tool-metadata-fact-flash-none = The camera recorded no flash
tool-metadata-fact-white_balance-label = White balance
tool-metadata-fact-white_balance-tag = WhiteBalance and LightSource, EXIF 0xA403, 0x9208
tool-metadata-fact-white_balance-none = The camera recorded no white balance
tool-metadata-fact-scene_type-label = Scene type
tool-metadata-fact-scene_type-tag = SceneCaptureType, EXIF 0xA406
tool-metadata-fact-scene_type-none = The camera recorded no scene type
tool-metadata-fact-subject_distance-label = Subject distance
tool-metadata-fact-subject_distance-tag = SubjectDistance and SubjectDistanceRange, EXIF 0x9206, 0xA40C
tool-metadata-fact-subject_distance-none = The camera recorded no subject distance
tool-metadata-fact-digital_zoom-label = Digital zoom
tool-metadata-fact-digital_zoom-tag = DigitalZoomRatio, EXIF 0xA404
tool-metadata-fact-digital_zoom-none = The camera recorded no digital zoom
tool-metadata-fact-contrast-label = Contrast
tool-metadata-fact-contrast-tag = Contrast, EXIF 0xA408
tool-metadata-fact-contrast-none = The camera recorded no contrast
tool-metadata-fact-saturation-label = Saturation
tool-metadata-fact-saturation-tag = Saturation, EXIF 0xA409
tool-metadata-fact-saturation-none = The camera recorded no saturation
tool-metadata-fact-sharpness-label = Sharpness
tool-metadata-fact-sharpness-tag = Sharpness, EXIF 0xA40A
tool-metadata-fact-sharpness-none = The camera recorded no sharpness
tool-metadata-fact-brightness-label = Brightness
tool-metadata-fact-brightness-tag = BrightnessValue, EXIF 0x9203
tool-metadata-fact-brightness-none = The camera recorded no brightness
tool-metadata-fact-colour_space-label = Colour space
tool-metadata-fact-colour_space-tag = ColorSpace, EXIF 0xA001
tool-metadata-fact-colour_space-none = The camera recorded no colour space
tool-metadata-fact-unique_image_id-label = Unique image ID
tool-metadata-fact-unique_image_id-tag = ImageUniqueID, EXIF 0xA420
tool-metadata-fact-unique_image_id-none = The camera recorded no unique image ID
tool-metadata-fact-lens-label = Lens
tool-metadata-fact-lens-tag = LensModel, EXIF 0xA434
tool-metadata-fact-lens-none = The camera recorded no lens
tool-metadata-fact-lens_make-label = Lens make
tool-metadata-fact-lens_make-tag = LensMake, EXIF 0xA433
tool-metadata-fact-lens_make-none = The camera recorded no lens make
tool-metadata-fact-lens_serial_number-label = Lens serial number
tool-metadata-fact-lens_serial_number-tag = LensSerialNumber, EXIF 0xA435
tool-metadata-fact-lens_serial_number-none = The camera recorded no lens serial number
tool-metadata-fact-lens_specification-label = Lens specification
tool-metadata-fact-lens_specification-tag = LensSpecification, EXIF 0xA432
tool-metadata-fact-lens_specification-none = The camera recorded no lens specification
tool-metadata-fact-focal_length-label = Focal length
tool-metadata-fact-focal_length-tag = FocalLength, EXIF 0x920A
tool-metadata-fact-focal_length-none = The camera recorded no focal length
tool-metadata-fact-focal_length_35mm-label = 35 mm equivalent
tool-metadata-fact-focal_length_35mm-tag = FocalLengthIn35mmFilm, EXIF 0xA405
tool-metadata-fact-focal_length_35mm-none = Neither a 35 mm equivalent nor a crop factor is recorded
tool-metadata-fact-crop_factor-label = Crop factor
tool-metadata-fact-crop_factor-tag = Worked out as darktable does, from FocalLengthIn35mmFilm or the focal plane’s resolution
tool-metadata-fact-crop_factor-none = Nothing the camera recorded gives a crop factor
tool-metadata-fact-max_aperture-label = Maximum aperture
tool-metadata-fact-max_aperture-tag = MaxApertureValue, EXIF 0x9205
tool-metadata-fact-max_aperture-none = The camera recorded no maximum aperture
tool-metadata-fact-latitude-label = Latitude
tool-metadata-fact-latitude-tag = GPSLatitude and GPSLatitudeRef, EXIF GPS 0x0002, 0x0001
tool-metadata-fact-latitude-none = The camera recorded no location
tool-metadata-fact-longitude-label = Longitude
tool-metadata-fact-longitude-tag = GPSLongitude and GPSLongitudeRef, EXIF GPS 0x0004, 0x0003
tool-metadata-fact-longitude-none = The camera recorded no location
tool-metadata-fact-altitude-label = Altitude
tool-metadata-fact-altitude-tag = GPSAltitude and GPSAltitudeRef, EXIF GPS 0x0006, 0x0005
tool-metadata-fact-altitude-none = The camera recorded no altitude
tool-metadata-fact-direction-label = Direction
tool-metadata-fact-direction-tag = GPSImgDirection and GPSImgDirectionRef, EXIF GPS 0x0011, 0x0010
tool-metadata-fact-direction-none = The camera recorded no direction
tool-metadata-fact-gps_time-label = GPS time
tool-metadata-fact-gps_time-tag = GPSDateStamp and GPSTimeStamp, EXIF GPS 0x001D, 0x0007
tool-metadata-fact-gps_time-none = The camera recorded no GPS time
tool-metadata-fact-speed-label = Speed
tool-metadata-fact-speed-tag = GPSSpeed and GPSSpeedRef, EXIF GPS 0x000D, 0x000C
tool-metadata-fact-speed-none = The camera recorded no speed
tool-metadata-fact-accuracy-label = Accuracy
tool-metadata-fact-accuracy-tag = GPSHPositioningError, EXIF GPS 0x001F
tool-metadata-fact-accuracy-none = The camera recorded no accuracy
tool-metadata-fact-satellites-label = Satellites
tool-metadata-fact-satellites-tag = GPSSatellites, EXIF GPS 0x0008
tool-metadata-fact-satellites-none = The camera recorded no satellites
tool-metadata-fact-map_datum-label = Map datum
tool-metadata-fact-map_datum-tag = GPSMapDatum, EXIF GPS 0x0012
tool-metadata-fact-map_datum-none = The camera recorded no map datum
tool-metadata-fact-rating-label = Rating
tool-metadata-fact-colour_labels-label = Colour labels
