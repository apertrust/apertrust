# The tools' words in American English (docs/gui.md, Tool panels from definitions): an
# overlay of en-GB/tools.ftl, not a copy. It holds only the messages whose words American
# English spells differently, with en-GB's variables, among them darktable's module names,
# which are American upstream (Color balance RGB); every other message is en-GB's.

## Highlight reconstruction

tool-highlights-control-combine-promise = Combine closely related clipped segments by morphological operations. This often leads to improved color reconstruction for tiny segments before dark background.
tool-highlights-control-candidating-promise = Select inpainting after segmentation analysis. Increase to favor candidates found in segmentation analysis, decrease for opposed means inpainting.
tool-highlights-control-solid_color-label = Inpaint a flat color
tool-highlights-control-solid_color-full = Inpaint a flat color

## Demosaic

tool-demosaic-control-demosaicing_method-option-photosite_color = Photosite color (debug)
tool-demosaic-control-demosaicing_method-option-photosite_color_xtrans = Photosite color (debug)
tool-demosaic-control-color_smoothing-label = Color smoothing
tool-demosaic-control-color_smoothing-full = Color smoothing
tool-demosaic-control-color_smoothing-promise = How many color smoothing median steps after demosaicing.
tool-demosaic-control-cs_radius-promise = Capture sharpen radius should reflect the overall Gaussian-type blur of the camera sensor, possibly the anti-aliasing filter and the lens. Increasing this too far will soon lead to artifacts like halos and ringing, especially when used with a large iterations setting. A radius set to zero will be recalculated automatically the next run; use for presets.
tool-demosaic-control-cs_boost-promise = Further increase sharpen radius at image corners; the sharp center of the image will not be affected.
tool-demosaic-control-cs_center-label = Sharp center
tool-demosaic-control-cs_center-full = Capture sharpen sharp center
tool-demosaic-control-cs_center-promise = Adjust to the sharp image center.

## AgX

tool-agx-control-range_white_relative_ev-promise = Relative exposure above mid-gray (white point).
tool-agx-control-range_black_relative_ev-promise = Relative exposure below mid-gray (black point).
tool-agx-control-curve_pivot_x-promise = Set the pivot's input exposure in EV relative to mid-gray.
tool-agx-control-base_primaries-promise = Color space primaries to use as the base for the adjustments below. Export profile uses the profile set in Output color profile.
tool-agx-control-red_unrotation-promise = Reverse the color shift in reds.
tool-agx-control-green_unrotation-promise = Reverse the color shift in greens.
tool-agx-control-blue_unrotation-promise = Reverse the color shift in blues.

## Input color profile

tool-colorin-title = Input color profile
    .search = input color profile
tool-colorin-control-type-promise = The color profile the camera's colors are read in.
tool-colorin-control-type-option-standard_matrix = Standard color matrix
tool-colorin-control-type_work-promise = The color space every tool after this one works in.
tool-colorin-control-normalize-promise = Confine Lab values to gamut of RGB color space.

## Output color profile

tool-colorout-title = Output color profile
    .search = output color profile
tool-colorout-control-type-promise = The color profile the image is exported in.

## White balance

tool-temperature-control-temperature-promise = The color temperature, in kelvin.
tool-temperature-control-tint-promise = The color tint of the image, from magenta (below 1) to green (above 1).
tool-temperature-control-various-promise = What a four-color sensor's emerald photosites are multiplied by.

## Orientation

tool-flip-action-rotate_ccw-label = Rotate 90° counterclockwise
tool-flip-action-rotate_ccw-full = Rotate 90° counterclockwise
tool-flip-action-rotate_ccw-promise = Rotate the image 90° counterclockwise.

## Exposure

tool-exposure-setting-target-option-middle_grey = Middle gray
tool-exposure-picker-exposure-promise-middle_grey = Make an area middle gray
tool-exposure-picker-exposure-hint-middle_grey = Make an area middle gray

## Color calibration

tool-channelmixerrgb-title = Color calibration
    .search = color calibration|channel mixer|white balance|monochrome
tool-channelmixerrgb-control-adaptation-promise = How the illuminant is adapted, and the color space the tool works in. Linear Bradford (1985) is consistent with the ICC v4 toolchain. CAT16 (2016) is more robust and accurate. Non-linear Bradford (1985) is the original Bradford: it can give better results than the linear one, but is unreliable. XYZ is a simple scaling in XYZ space, not recommended in general. None disables any adaptation and uses the pipeline's working RGB.
tool-channelmixerrgb-control-illuminant-promise = The light the scene was lit by, which the adaptation turns into white: a standard light, one at a temperature, the camera's own or a custom color.
tool-channelmixerrgb-control-hue-promise = The color of the light, around the pipeline's white.
tool-channelmixerrgb-control-gamut-promise = Brings colors outside the pipeline's gamut back inside it, the more the higher. 0 = none.
tool-channelmixerrgb-collapsible-area_colour_mapping = Area color mapping
tool-channelmixerrgb-setting-area_mode-promise = Correction adjusts the illuminant so the input color is mapped to the target. Measure only shows how an input color is mapped by the CAT, and can be used to sample a target.
tool-channelmixerrgb-setting-target_lightness-promise = The lightness of the desired target color after mapping.
tool-channelmixerrgb-setting-target_hue-promise = The hue of the desired target color after mapping.
tool-channelmixerrgb-setting-target_chroma-promise = The chroma of the desired target color after mapping.
tool-channelmixerrgb-tab-colourfulness = Colorfulness
tool-channelmixerrgb-tab-grey = Gray
tool-channelmixerrgb-control-red_normalise-label = Normalize channels
tool-channelmixerrgb-control-red_normalise-full = Red, normalize channels
tool-channelmixerrgb-control-green_normalise-label = Normalize channels
tool-channelmixerrgb-control-green_normalise-full = Green, normalize channels
tool-channelmixerrgb-control-blue_normalise-label = Normalize channels
tool-channelmixerrgb-control-blue_normalise-full = Blue, normalize channels
tool-channelmixerrgb-control-colourfulness_r-full = Colorfulness, input R
tool-channelmixerrgb-control-colourfulness_r-promise = Makes colors more or less colorful by their R, keeping their brightness. 0 = untouched.
tool-channelmixerrgb-control-colourfulness_g-full = Colorfulness, input G
tool-channelmixerrgb-control-colourfulness_g-promise = Makes colors more or less colorful by their G, keeping their brightness. 0 = untouched.
tool-channelmixerrgb-control-colourfulness_b-full = Colorfulness, input B
tool-channelmixerrgb-control-colourfulness_b-promise = Makes colors more or less colorful by their B, keeping their brightness. 0 = untouched.
tool-channelmixerrgb-control-colourfulness_normalise-label = Normalize channels
tool-channelmixerrgb-control-colourfulness_normalise-full = Colorfulness, normalize channels
tool-channelmixerrgb-control-colourfulness_normalise-promise = Takes the three weights' mean from each, so the average color keeps its colorfulness.
tool-channelmixerrgb-control-brightness_normalise-label = Normalize channels
tool-channelmixerrgb-control-brightness_normalise-full = Brightness, normalize channels
tool-channelmixerrgb-control-brightness_normalise-promise = Takes the three weights' mean from each, so the average color keeps its brightness.
tool-channelmixerrgb-control-grey_r-full = Gray, input R
tool-channelmixerrgb-control-grey_r-promise = How much of the input's R the gray takes. Any weight off 0 makes the photo gray. 0 = untouched.
tool-channelmixerrgb-control-grey_g-full = Gray, input G
tool-channelmixerrgb-control-grey_g-promise = How much of the input's G the gray takes. Any weight off 0 makes the photo gray. 0 = untouched.
tool-channelmixerrgb-control-grey_b-full = Gray, input B
tool-channelmixerrgb-control-grey_b-promise = How much of the input's B the gray takes. Any weight off 0 makes the photo gray. 0 = untouched.
tool-channelmixerrgb-control-grey_normalise-label = Normalize channels
tool-channelmixerrgb-control-grey_normalise-full = Gray, normalize channels
tool-channelmixerrgb-control-grey_normalise-promise = Scales the gray's three weights so they add up to 1, keeping the photo's brightness.
tool-channelmixerrgb-collapsible-colour_checker = Calibrate with a color checker
tool-channelmixerrgb-setting-chart-full = Color checker chart
tool-channelmixerrgb-setting-optimise_for-label = Optimize for
tool-channelmixerrgb-setting-optimise_for-full = Colors the color checker favors
tool-channelmixerrgb-setting-optimise_for-promise = The colors the fit gets closest. Neutral colors give the lowest average ΔE, saturated colors the lowest maximum.
tool-channelmixerrgb-setting-optimise_for-option-neutral = Neutral colors
tool-channelmixerrgb-setting-optimise_for-option-saturated = Saturated colors
tool-channelmixerrgb-setting-optimise_for-option-skin = Skin and soil colors
tool-channelmixerrgb-setting-optimise_for-option-foliage = Foliage colors
tool-channelmixerrgb-setting-optimise_for-option-sky = Sky and water colors
tool-channelmixerrgb-setting-patch_scale-full = Color checker patch scale
tool-channelmixerrgb-setting-patch_scale-promise = How much of each patch is read, from its center: less where the chart's frame is loose or casts shadows.

## Tone equalizer

tool-toneequal-control-quantization-label = Mask quantization
tool-toneequal-control-quantization-full = Mask quantization
tool-toneequal-control-quantization-promise = 0 turns quantization off. Higher values posterize the luminance mask, helping the filter produce piecewise smooth areas with high feathering values.

## Color balance RGB

tool-colorbalancergb-title = Color balance RGB
    .search = color balance rgb|offset power slope|cdl|color grading|contrast|chroma_highlights|hue|vibrance|saturation
tool-colorbalancergb-control-vibrance-promise = Increases colorfulness mostly on low-chroma colors.
tool-colorbalancergb-control-chroma_global-promise = Increases colorfulness at the same luminance, globally.
tool-colorbalancergb-control-chroma_shadows-promise = Increases colorfulness at the same luminance, mostly in the shadows.
tool-colorbalancergb-control-chroma_midtones-promise = Increases colorfulness at the same luminance, mostly in the mid-tones.
tool-colorbalancergb-control-chroma_highlights-promise = Increases colorfulness at the same luminance, mostly in the highlights.
tool-colorbalancergb-control-global_h-promise = Hue of the global color offset.
tool-colorbalancergb-control-global_c-promise = Chroma of the global color offset.
tool-colorbalancergb-control-shadows_h-promise = Hue of the color gain in the shadows.
tool-colorbalancergb-control-shadows_c-promise = Chroma of the color gain in the shadows.
tool-colorbalancergb-control-highlights_h-promise = Hue of the color gain in the highlights.
tool-colorbalancergb-control-highlights_c-promise = Chroma of the color gain in the highlights.
tool-colorbalancergb-control-midtones_h-promise = Hue of the color exponent in the mid-tones.
tool-colorbalancergb-control-midtones_c-promise = Chroma of the color exponent in the mid-tones.
tool-colorbalancergb-control-saturation_formula-promise = Chooses the uniform color space saturation is computed in.
tool-colorbalancergb-control-mask_grey_fulcrum-label = Mask middle-gray fulcrum
tool-colorbalancergb-control-mask_grey_fulcrum-full = Mask middle-gray fulcrum
tool-colorbalancergb-control-mask_grey_fulcrum-promise = Position of the middle-gray reference for masking.
tool-colorbalancergb-control-white_fulcrum-promise = Peak white luminance used to normalize the power function.
tool-colorbalancergb-control-grey_fulcrum-label = Contrast gray fulcrum
tool-colorbalancergb-control-grey_fulcrum-full = Contrast gray fulcrum
tool-colorbalancergb-control-grey_fulcrum-promise = Peak gray luminance used to normalize the power function: the gray Contrast keeps.

## Tone curve

tool-tone_curve-control-luma_points-promise = Moves each tone to the height you draw it at. Keeps colors. A straight line = untouched.
tool-tone_curve-control-preserve_colour-label = Preserve colors
tool-tone_curve-control-preserve_colour-full = Preserve colors in the Luma curve
tool-tone_curve-control-preserve_colour-promise = What the Luma curve acts on in each pixel, scaling its three channels alike so its color stays.

## Grain

tool-grain-control-midtones_bias-promise = Amount of mid-tones bias from the photographic paper response modeling. The greater the bias, the more pronounced the fall-off of the grain in shadows and highlights.

## Vignetting

tool-vignette-control-center_x-label = Horizontal center
tool-vignette-control-center_x-full = Horizontal center
tool-vignette-control-center_x-promise = Horizontal offset of center of the effect.
tool-vignette-control-center_y-label = Vertical center
tool-vignette-control-center_y-full = Vertical center
tool-vignette-control-center_y-promise = Vertical offset of center of the effect.

# Pickers' words that American English spells differently.
tool-colorbalancergb-picker-global_hue-promise = Picks the color opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-shadows_hue-promise = Picks the color opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-highlights_hue-promise = Picks the color opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-midtones_hue-promise = Picks the color opposite the selected area's. Ctrl-click, Cmd-click on the Mac, picks the area's own.
tool-colorbalancergb-picker-white_fulcrum-promise = Pick color from the photo
tool-colorbalancergb-picker-grey_fulcrum-full = Contrast gray fulcrum: pick from the photo
tool-colorbalancergb-picker-grey_fulcrum-promise = Pick color from the photo
tool-agx-picker-pivot-promise = Sets the pivot relative to mid-gray from the average luminance of the selected area.
tool-agx-picker-pivot_output-promise = Sets the pivot relative to mid-gray from the average luminance of the selected area, and adjusts the output by the default mid-gray to mid-gray mapping.

## What each tool and utility is for (docs/gui.md, The header's tooltip)

tool-highlights-description = Keeps clipped highlights from turning magenta and tries to recover their color. Reach for it when the brightest areas look flat or tinted.
tool-demosaic-description = Turns the sensor’s mosaic of red, green and blue into full-color pixels. Usually left as it is: change it when fine detail shows mazes or false color.
tool-agx-description = Fits the light the camera caught into what a screen or a print can show, with natural highlights and colors. The default tone mapper, inspired by Blender’s AgX.
tool-colorin-description = Takes the camera’s colors into the color space every later tool works in. Usually left at the camera’s own matrix.
tool-colorout-description = Converts the finished picture into the colors of the file you export. Usually left at sRGB, which suits screens and the web.
tool-temperature-description = Balances the camera’s red, green and blue so that white comes out white, and helps demosaic. Usually left as it is: set the color of the light in Color calibration.
tool-exposure-description = Brightens or darkens the photo as if you had exposed it differently in the camera, keeping its colors. Usually the first tool to reach for.
tool-channelmixerrgb-description = Sets the color of the light, mixes the color channels, or turns the photo black and white as film would. Reach for it to remove a color cast.
tool-colorbalancergb-description = Color grading: shifts the color of the shadows, mid-tones and highlights separately, each picked out by a soft mask. Reach for it to give a photo its color mood.
tool-hazeremoval-description = Removes haze and fog, bringing back contrast and color in distant parts of the scene. Reach for it on landscapes that look washed out.
tool-primaries-description = Moves the red, green and blue primaries, shifting every color made of them. Reach for it for broad, smooth color grading.
tool-vignette-description = Darkens or lightens the photo towards its edges, as a lens does. Reach for it to draw the eye to the center.
tool-scopes-description = Shows how the photo’s tones and colors are spread: histogram, waveform, RGB parade or vectorscope. Reach for it to judge exposure and color as you edit.
