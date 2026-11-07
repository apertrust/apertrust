# The modules' words in American English (docs/gui.md, Module panels from definitions): an
# overlay of en-GB/modules.ftl, not a copy. It holds only the messages whose words American
# English spells differently, with en-GB's variables, among them darktable's module names,
# which are American upstream (Color balance RGB); every other message is en-GB's.

## Highlight reconstruction

module-highlights-control-combine-promise = Combine closely related clipped segments by morphological operations. This often leads to improved color reconstruction for tiny segments before dark background.
module-highlights-control-candidating-promise = Select inpainting after segmentation analysis. Increase to favor candidates found in segmentation analysis, decrease for opposed means inpainting.
module-highlights-control-solid_color-label = Inpaint a flat color
module-highlights-control-solid_color-full = Inpaint a flat color

## Demosaic

module-demosaic-control-demosaicing_method-option-photosite_color = Photosite color (debug)
module-demosaic-control-demosaicing_method-option-photosite_color_xtrans = Photosite color (debug)
module-demosaic-control-color_smoothing-label = Color smoothing
module-demosaic-control-color_smoothing-full = Color smoothing
module-demosaic-control-color_smoothing-promise = How many color smoothing median steps after demosaicing.
module-demosaic-control-cs_radius-promise = Capture sharpen radius should reflect the overall Gaussian-type blur of the camera sensor, possibly the anti-aliasing filter and the lens. Increasing this too far will soon lead to artifacts like halos and ringing, especially when used with a large iterations setting. A radius set to zero will be recalculated automatically the next run; use for presets.
module-demosaic-control-cs_boost-promise = Further increase sharpen radius at image corners; the sharp center of the image will not be affected.
module-demosaic-control-cs_center-label = Sharp center
module-demosaic-control-cs_center-full = Capture sharpen sharp center
module-demosaic-control-cs_center-promise = Adjust to the sharp image center.

## AgX

module-agx-control-range_white_relative_ev-promise = Relative exposure above mid-gray (white point).
module-agx-control-range_black_relative_ev-promise = Relative exposure below mid-gray (black point).
module-agx-control-curve_pivot_x-promise = Set the pivot's input exposure in EV relative to mid-gray.
module-agx-control-base_primaries-promise = Color space primaries to use as the base for the adjustments below. Export profile uses the profile set in Output color profile.
module-agx-control-red_unrotation-promise = Reverse the color shift in reds.
module-agx-control-green_unrotation-promise = Reverse the color shift in greens.
module-agx-control-blue_unrotation-promise = Reverse the color shift in blues.

## Output color profile

module-colorout-title = Output color profile
module-colorout-control-type-promise = The color profile the image is exported in.

## White balance

module-temperature-control-temperature-promise = The color temperature, in kelvin.
module-temperature-control-tint-promise = The color tint of the image, from magenta (below 1) to green (above 1).
module-temperature-control-various-promise = What a four-color sensor's emerald photosites are multiplied by.

## Orientation

module-flip-action-rotate_ccw-label = Rotate 90° counterclockwise
module-flip-action-rotate_ccw-full = Rotate 90° counterclockwise
module-flip-action-rotate_ccw-promise = Rotate the image 90° counterclockwise.

## Exposure

module-exposure-auto = Sets Exposure so the photo's tones sit around middle gray. A kept Exposure stays as it is.
module-exposure-setting-area_mode-promise = Correction adjusts the exposure so that the input's lightness is mapped to the target. Measure only shows how an input color is mapped by the exposure, and can be used to define a target.

## Color calibration

module-channelmixerrgb-title = Color calibration
module-channelmixerrgb-control-adaptation-promise = How the illuminant is adapted, and the color space the module works in. Linear Bradford (1985) is consistent with the ICC v4 toolchain. CAT16 (2016) is more robust and accurate. Non-linear Bradford (1985) is the original Bradford: it can give better results than the linear one, but is unreliable. XYZ is a simple scaling in XYZ space, not recommended in general. None disables any adaptation and uses the pipeline's working RGB.
module-channelmixerrgb-control-illuminant-promise = The light the scene was lit by, which the adaptation turns into white: a standard light, one at a temperature, the camera's own or a custom color.
module-channelmixerrgb-control-hue-promise = The color of the light, around the pipeline's white.
module-channelmixerrgb-control-gamut-promise = Brings colors outside the pipeline's gamut back inside it, the more the higher. 0 = none.
module-channelmixerrgb-collapsible-area_colour_mapping = Area color mapping
module-channelmixerrgb-setting-area_mode-promise = Correction adjusts the illuminant so the input color is mapped to the target. Measure only shows how an input color is mapped by the CAT, and can be used to sample a target.
module-channelmixerrgb-setting-target_lightness-promise = The lightness of the desired target color after mapping.
module-channelmixerrgb-setting-target_hue-promise = The hue of the desired target color after mapping.
module-channelmixerrgb-setting-target_chroma-promise = The chroma of the desired target color after mapping.
module-channelmixerrgb-tab-colourfulness = Colorfulness
module-channelmixerrgb-tab-grey = Gray
module-channelmixerrgb-control-red_normalise-label = Normalize channels
module-channelmixerrgb-control-red_normalise-full = Red, normalize channels
module-channelmixerrgb-control-green_normalise-label = Normalize channels
module-channelmixerrgb-control-green_normalise-full = Green, normalize channels
module-channelmixerrgb-control-blue_normalise-label = Normalize channels
module-channelmixerrgb-control-blue_normalise-full = Blue, normalize channels
module-channelmixerrgb-control-colourfulness_r-full = Colorfulness, input R
module-channelmixerrgb-control-colourfulness_r-promise = Makes colors more or less colorful by their R, keeping their brightness. 0 = untouched.
module-channelmixerrgb-control-colourfulness_g-full = Colorfulness, input G
module-channelmixerrgb-control-colourfulness_g-promise = Makes colors more or less colorful by their G, keeping their brightness. 0 = untouched.
module-channelmixerrgb-control-colourfulness_b-full = Colorfulness, input B
module-channelmixerrgb-control-colourfulness_b-promise = Makes colors more or less colorful by their B, keeping their brightness. 0 = untouched.
module-channelmixerrgb-control-colourfulness_normalise-label = Normalize channels
module-channelmixerrgb-control-colourfulness_normalise-full = Colorfulness, normalize channels
module-channelmixerrgb-control-colourfulness_normalise-promise = Takes the three weights' mean from each, so the average color keeps its colorfulness.
module-channelmixerrgb-control-brightness_normalise-label = Normalize channels
module-channelmixerrgb-control-brightness_normalise-full = Brightness, normalize channels
module-channelmixerrgb-control-brightness_normalise-promise = Takes the three weights' mean from each, so the average color keeps its brightness.
module-channelmixerrgb-control-grey_r-full = Gray, input R
module-channelmixerrgb-control-grey_r-promise = How much of the input's R the gray takes. Any weight off 0 makes the photo gray. 0 = untouched.
module-channelmixerrgb-control-grey_g-full = Gray, input G
module-channelmixerrgb-control-grey_g-promise = How much of the input's G the gray takes. Any weight off 0 makes the photo gray. 0 = untouched.
module-channelmixerrgb-control-grey_b-full = Gray, input B
module-channelmixerrgb-control-grey_b-promise = How much of the input's B the gray takes. Any weight off 0 makes the photo gray. 0 = untouched.
module-channelmixerrgb-control-grey_normalise-label = Normalize channels
module-channelmixerrgb-control-grey_normalise-full = Gray, normalize channels
module-channelmixerrgb-control-grey_normalise-promise = Scales the gray's three weights so they add up to 1, keeping the photo's brightness.
module-channelmixerrgb-collapsible-colour_checker = Calibrate with a color checker
module-channelmixerrgb-setting-chart-full = Color checker chart
module-channelmixerrgb-setting-optimise_for-label = Optimize for
module-channelmixerrgb-setting-optimise_for-full = Colors the color checker favors
module-channelmixerrgb-setting-optimise_for-promise = The colors the fit gets closest. Neutral colors give the lowest average ΔE, saturated colors the lowest maximum.
module-channelmixerrgb-setting-optimise_for-option-neutral = Neutral colors
module-channelmixerrgb-setting-optimise_for-option-saturated = Saturated colors
module-channelmixerrgb-setting-optimise_for-option-skin = Skin and soil colors
module-channelmixerrgb-setting-optimise_for-option-foliage = Foliage colors
module-channelmixerrgb-setting-optimise_for-option-sky = Sky and water colors
module-channelmixerrgb-setting-patch_scale-full = Color checker patch scale
module-channelmixerrgb-setting-patch_scale-promise = How much of each patch is read, from its center: less where the chart's frame is loose or casts shadows.

## Tone equalizer

module-toneequal-control-quantization-label = Mask quantization
module-toneequal-control-quantization-full = Mask quantization
module-toneequal-control-quantization-promise = 0 turns quantization off. Higher values posterize the luminance mask, helping the filter produce piecewise smooth areas with high feathering values.

## Color balance RGB

module-colorbalancergb-title = Color balance RGB
module-colorbalancergb-control-vibrance-promise = Increases colorfulness mostly on low-chroma colors.
module-colorbalancergb-control-chroma_global-promise = Increases colorfulness at the same luminance, globally.
module-colorbalancergb-control-chroma_shadows-promise = Increases colorfulness at the same luminance, mostly in the shadows.
module-colorbalancergb-control-chroma_midtones-promise = Increases colorfulness at the same luminance, mostly in the mid-tones.
module-colorbalancergb-control-chroma_highlights-promise = Increases colorfulness at the same luminance, mostly in the highlights.
module-colorbalancergb-control-global_h-promise = Hue of the global color offset.
module-colorbalancergb-control-global_c-promise = Chroma of the global color offset.
module-colorbalancergb-control-shadows_h-promise = Hue of the color gain in the shadows.
module-colorbalancergb-control-shadows_c-promise = Chroma of the color gain in the shadows.
module-colorbalancergb-control-highlights_h-promise = Hue of the color gain in the highlights.
module-colorbalancergb-control-highlights_c-promise = Chroma of the color gain in the highlights.
module-colorbalancergb-control-midtones_h-promise = Hue of the color exponent in the mid-tones.
module-colorbalancergb-control-midtones_c-promise = Chroma of the color exponent in the mid-tones.
module-colorbalancergb-control-saturation_formula-promise = Chooses the uniform color space saturation is computed in.
module-colorbalancergb-control-mask_grey_fulcrum-label = Mask middle-gray fulcrum
module-colorbalancergb-control-mask_grey_fulcrum-full = Mask middle-gray fulcrum
module-colorbalancergb-control-mask_grey_fulcrum-promise = Position of the middle-gray reference for masking.
module-colorbalancergb-control-white_fulcrum-promise = Peak white luminance used to normalize the power function.
module-colorbalancergb-control-grey_fulcrum-label = Contrast gray fulcrum
module-colorbalancergb-control-grey_fulcrum-full = Contrast gray fulcrum
module-colorbalancergb-control-grey_fulcrum-promise = Peak gray luminance used to normalize the power function: the gray Contrast keeps.

## Tone curve

module-tone_curve-control-luma_points-promise = Moves each tone to the height you draw it at. Keeps colors. A straight line = untouched.
module-tone_curve-control-preserve_colour-label = Preserve colors
module-tone_curve-control-preserve_colour-full = Preserve colors in the Luma curve
module-tone_curve-control-preserve_colour-promise = What the Luma curve acts on in each pixel, scaling its three channels alike so its color stays.

## Grain

module-grain-control-midtones_bias-promise = Amount of mid-tones bias from the photographic paper response modeling. The greater the bias, the more pronounced the fall-off of the grain in shadows and highlights.

## Vignetting

module-vignette-control-center_x-label = Horizontal center
module-vignette-control-center_x-full = Horizontal center
module-vignette-control-center_x-promise = Horizontal offset of center of the effect.
module-vignette-control-center_y-label = Vertical center
module-vignette-control-center_y-full = Vertical center
module-vignette-control-center_y-promise = Vertical offset of center of the effect.
