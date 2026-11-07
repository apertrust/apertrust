// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2025  darktable developers (ported from
// src/common/colorspaces_inline_conversions.h, darktable_ucs_22_helpers.h,
// rgb_norms.h, illuminants.h and math.h)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// Shared colour conversions, for every operation's kernel. Matrices are
// written row by row and applied as mul(M, v). Functions are differentiable so kernels
// can backpropagate through them.

module color;

public static const float PI = 3.14159265358979323846;
public static const float HALF_PI = 1.57079632679489661923;

// pow for a non-negative base and positive exponent that returns 0 at 0 instead of
// relying on log2(0) = -inf.
//
// Its derivatives are written out: Slang's own derivative of pow is zero for bases below
// 1e-6, where colour maths spends much of its time (the JzAzBz curve divides XYZ by
// 10000), and where the derivative of a power below 1 is large rather than small.
[ForwardDerivative(pow0_forward)]
[BackwardDerivative(pow0_backward)]
public float pow0(float x, float y)
{
    if (x <= 0.0)
        return 0.0;
    return pow(x, y);
}

DifferentialPair<float> pow0_forward(DifferentialPair<float> x, DifferentialPair<float> y)
{
    if (x.p <= 0.0)
        return diffPair(0.0, 0.0);
    float value = pow(x.p, y.p);
    return diffPair(value, x.d * y.p * pow(x.p, y.p - 1.0) + y.d * value * log(x.p));
}

void pow0_backward(inout DifferentialPair<float> x, inout DifferentialPair<float> y, float d_out)
{
    if (x.p <= 0.0)
    {
        x = diffPair(x.p, 0.0);
        y = diffPair(y.p, 0.0);
        return;
    }
    float value = pow(x.p, y.p);
    float d_x = d_out * y.p * pow(x.p, y.p - 1.0);
    float d_y = d_out * value * log(x.p);
    x = diffPair(x.p, d_x);
    y = diffPair(y.p, d_y);
}

[Differentiable]
public float3 pow0(float3 x, float3 y)
{
    return float3(pow0(x.x, y.x), pow0(x.y, y.y), pow0(x.z, y.z));
}

// Filmlight grading RGB and CIE 2006 LMS D65.
public static const float3x3 GRADING_RGB_TO_LMS = float3x3(
    0.95, 0.38, 0.0,
    0.05, 0.62, 0.03,
    0.0, 0.0, 0.97);
public static const float3x3 LMS_TO_GRADING_RGB = float3x3(
    1.0877193, -0.66666667, 0.02061856,
    -0.0877193, 1.66666667, -0.05154639,
    0.0, 0.0, 1.03092784);
public static const float3x3 LMS_2006_TO_XYZ_D65 = float3x3(
    1.80794659, -1.29971660, 0.34785879,
    0.61783960, 0.39595453, -0.04104687,
    -0.12546960, 0.20478038, 1.74274183);

// (r, g) chromaticity of white in Filmlight Yrg.
public static const float2 YRG_WHITE = float2(0.21902143, 0.54371398);

[Differentiable]
public float3 lms_to_yrg(float3 lms)
{
    float y = 0.68990272 * lms.x + 0.34832189 * lms.y;
    float a = lms.x + lms.y + lms.z;
    float3 normalized = float3(0.0);
    if (a != 0.0)
        normalized = lms / a;
    float3 rgb = mul(LMS_TO_GRADING_RGB, normalized);
    return float3(y, rgb.x, rgb.y);
}

[Differentiable]
public float3 yrg_to_lms(float3 yrg)
{
    float3 lms = mul(GRADING_RGB_TO_LMS, float3(yrg.y, yrg.z, 1.0 - yrg.y - yrg.z));
    float denominator = 0.68990272 * lms.x + 0.34832189 * lms.y;
    float a = 0.0;
    if (denominator != 0.0)
        a = yrg.x / denominator;
    return lms * a;
}

// Ych: luminance, chroma, and the hue as its cosine and sine.
[Differentiable]
public float4 yrg_to_ych(float3 yrg)
{
    float2 rg = yrg.yz - YRG_WHITE;
    float c = length(rg);
    float2 cos_sin = float2(1.0, 0.0);
    if (c != 0.0)
        cos_sin = rg / c;
    return float4(yrg.x, c, cos_sin.x, cos_sin.y);
}

[Differentiable]
public float3 ych_to_yrg(float4 ych)
{
    float2 rg = ych.y * ych.zw + YRG_WHITE;
    return float3(ych.x, rg.x, rg.y);
}

// Clips chroma at constant luminance and hue so the colour stays inside Yrg.
[Differentiable]
public float4 gamut_check_yrg(float4 ych)
{
    float3 yrg = ych_to_yrg(ych);
    float max_c = ych.y;
    if (yrg.y < 0.0)
        max_c = min(-YRG_WHITE.x / ych.z, max_c);
    if (yrg.z < 0.0)
        max_c = min(-YRG_WHITE.y / ych.w, max_c);
    if (yrg.y + yrg.z > 1.0)
        max_c = min((1.0 - YRG_WHITE.x - YRG_WHITE.y) / (ych.z + ych.w), max_c);
    return float4(ych.x, max_c, ych.z, ych.w);
}

// JzAzBz (Safdar et al. 2017), from and to XYZ D65.
static const float JZ_B = 1.15;
static const float JZ_G = 0.66;
static const float JZ_C1 = 0.8359375;
static const float JZ_C2 = 18.8515625;
static const float JZ_C3 = 18.6875;
static const float JZ_N = 0.159301758;
static const float JZ_P = 134.034375;
static const float JZ_D = -0.56;
static const float JZ_D0 = 1.6295499532821566e-11;
static const float3x3 JZ_XYZ_TO_LMS = float3x3(
    0.41478972, 0.57999900, 0.01464800,
    -0.2015100, 1.1206490, 0.0531008,
    -0.0166008, 0.2648000, 0.6684799);
static const float3x3 JZ_LMS_TO_IAB = float3x3(
    0.5, 0.5, 0.0,
    3.524000, -4.066708, 0.542708,
    0.199076, 1.096799, -1.295875);
public static const float3x3 JZ_IAB_TO_LMS = float3x3(
    1.0, 0.1386050432715393, 0.0580473161561189,
    1.0, -0.1386050432715393, -0.0580473161561189,
    1.0, -0.0960192420263190, -0.8118918960560390);
static const float3x3 JZ_LMS_TO_XYZ = float3x3(
    1.9242264357876067, -1.0047923125953657, 0.0376514040306180,
    0.3503167620949991, 0.7264811939316552, -0.0653844229480850,
    -0.0909828109828475, -0.3127282905230739, 1.5227665613052603);

[Differentiable]
public float3 xyz_to_jzazbz(float3 xyz)
{
    float3 xyz_prime = float3(
        JZ_B * xyz.x - (JZ_B - 1.0) * xyz.z,
        JZ_G * xyz.y - (JZ_G - 1.0) * xyz.x,
        xyz.z);
    float3 t = pow0(max(mul(JZ_XYZ_TO_LMS, xyz_prime) / 10000.0, float3(0.0)), float3(JZ_N));
    float3 lms = pow0((JZ_C1 + JZ_C2 * t) / (1.0 + JZ_C3 * t), float3(JZ_P));
    float3 iab = mul(JZ_LMS_TO_IAB, lms);
    float jz = max(((1.0 + JZ_D) * iab.x) / (1.0 + JZ_D * iab.x) - JZ_D0, 0.0);
    return float3(jz, iab.y, iab.z);
}

[Differentiable]
public float jz_to_iz(float jz)
{
    float iz = jz + JZ_D0;
    return max(iz / (1.0 + JZ_D - JZ_D * iz), 0.0);
}

[Differentiable]
public float3 jzazbz_to_xyz(float3 jab)
{
    float3 lms_prime = max(mul(JZ_IAB_TO_LMS, float3(jz_to_iz(jab.x), jab.y, jab.z)), float3(0.0));
    float3 t = pow0(lms_prime, float3(1.0 / JZ_P));
    float3 lms = 10000.0 * pow0(max((JZ_C1 - t) / (JZ_C3 * t - JZ_C2), float3(0.0)), float3(1.0 / JZ_N));
    float3 xyz_prime = mul(JZ_LMS_TO_XYZ, lms);
    float x = (xyz_prime.x + (JZ_B - 1.0) * xyz_prime.z) / JZ_B;
    float y = (xyz_prime.y + (JZ_G - 1.0) * x) / JZ_G;
    return float3(x, y, xyz_prime.z);
}

// CIE Lab with the D50 white, as darktable converts to and from it
// (src/common/colorspaces_inline_conversions.h): the ICC's D50, a cube root above
// (6/29)^3 and a line below, which meet with equal slopes. darktable's cbrtf is
// pow0(x, 1/3) here, its argument being above (6/29)^3.
static const float3 LAB_D50 = float3(0.9642, 1.0, 0.8249);
static const float LAB_EPSILON = 216.0 / 24389.0;
static const float LAB_KAPPA = 24389.0 / 27.0;
// cbrt(LAB_EPSILON), 6/29, as darktable writes it.
static const float LAB_F_EPSILON = 0.20689655172413796;

[Differentiable]
float lab_f(float x)
{
    if (x > LAB_EPSILON)
        return pow0(x, 1.0 / 3.0);
    return (LAB_KAPPA * x + 16.0) / 116.0;
}

[Differentiable]
float lab_f_inv(float x)
{
    if (x > LAB_F_EPSILON)
        return x * x * x;
    return (116.0 * x - 16.0) / LAB_KAPPA;
}

// darktable dt_XYZ_to_Lab.
[Differentiable]
public float3 xyz_to_lab_d50(float3 xyz)
{
    float3 x = xyz / LAB_D50;
    float3 f = float3(lab_f(x.x), lab_f(x.y), lab_f(x.z));
    return float3(116.0 * f.y - 16.0, 500.0 * (f.x - f.y), -200.0 * (f.z - f.y));
}

// darktable dt_Lab_to_XYZ.
[Differentiable]
public float3 lab_to_xyz_d50(float3 lab)
{
    float fy = (lab.x + 16.0) * (1.0 / 116.0);
    float fx = lab.y * (1.0 / 500.0) + fy;
    float fz = lab.z * (-1.0 / 200.0) + fy;
    return LAB_D50 * float3(lab_f_inv(fx), lab_f_inv(fy), lab_f_inv(fz));
}

// Exponential soft clipping above `soft`, approaching `hard`.
[Differentiable]
public float soft_clip(float x, float soft, float hard)
{
    if (x <= soft)
        return x;
    float norm = hard - soft;
    if (norm <= 0.0)
        return soft;
    return soft + (1.0 - exp(-(x - soft) / norm)) * norm;
}

// darktable's dt_iop_rgb_norms_t, the norms its tone curves preserve colours by.
public static const uint RGB_NORM_NONE = 0;
public static const uint RGB_NORM_LUMINANCE = 1;
public static const uint RGB_NORM_MAX = 2;
public static const uint RGB_NORM_AVERAGE = 3;
public static const uint RGB_NORM_SUM = 4;
public static const uint RGB_NORM_NORM = 5;
public static const uint RGB_NORM_POWER = 6;

// darktable dt_rgb_norm (src/common/rgb_norms.h): one of the norms above, `luminance`
// being the Y row of the working space's RGB to XYZ D50 matrix. Any other norm, none
// included, is the average, as in darktable. The basic power of black is 0, where
// darktable's divides 0 by 0.
[Differentiable]
public float rgb_norm(float3 rgb, no_diff uint norm, no_diff float3 luminance)
{
    if (norm == RGB_NORM_LUMINANCE)
        return dot(luminance, rgb);
    if (norm == RGB_NORM_MAX)
        return max(rgb.x, max(rgb.y, rgb.z));
    if (norm == RGB_NORM_SUM)
        return rgb.x + rgb.y + rgb.z;
    if (norm == RGB_NORM_NORM)
        return sqrt(dot(rgb, rgb));
    if (norm == RGB_NORM_POWER)
    {
        float3 squared = rgb * rgb;
        float sum = squared.x + squared.y + squared.z;
        return sum > 0.0 ? dot(rgb, squared) / sum : 0.0;
    }
    return (rgb.x + rgb.y + rgb.z) / 3.0;
}

// HSV, darktable dt_RGB_2_HSV: hue in [0, 1), saturation and value. Hue and saturation
// are 0 when the largest channel or the range is within 1e-6 of 0. Every candidate
// hue is computed and one selected, so no value the backward pass needs exists only
// inside a branch.
[Differentiable]
public float3 rgb_to_hsv(float3 rgb)
{
    float lo = min(rgb.x, min(rgb.y, rgb.z));
    float hi = max(rgb.x, max(rgb.y, rgb.z));
    float delta = hi - lo;
    bool chromatic = abs(hi) > 1e-6 && abs(delta) > 1e-6;
    float safe_delta = chromatic ? delta : 1.0;
    float safe_hi = chromatic ? hi : 1.0;
    float hue_r = (rgb.y - rgb.z) / safe_delta;
    float hue_g = 2.0 + (rgb.z - rgb.x) / safe_delta;
    float hue_b = 4.0 + (rgb.x - rgb.y) / safe_delta;
    float hue = (rgb.x == hi ? hue_r : (rgb.y == hi ? hue_g : hue_b)) / 6.0;
    hue = hue - no_diff floor(hue);
    return float3(chromatic ? hue : 0.0, chromatic ? delta / safe_hi : 0.0, hi);
}

// darktable dt_HSV_2_RGB, for a hue in [0, 1]. A hue of 1, which t - floor(t) gives for
// a tiny negative t, is hue 0 here; darktable puts it in its last sector, where it
// renders red as magenta.
//
// Its derivatives are written out. The function has a corner at every sector boundary,
// where Slang's own derivative takes the slope of the sector on the right; these take the
// average of the slopes either side, which is what central differences measure. A hue
// within HSV_BOUNDARY of a boundary counts as on it, since the backward pass recomputes
// the hue and on Metal the recomputation is not bit-identical.
[ForwardDerivative(hsv_to_rgb_forward)]
[BackwardDerivative(hsv_to_rgb_backward)]
public float3 hsv_to_rgb(float3 hsv)
{
    float base = floor(hsv.x * 6.0);
    return hsv_in_sector(hsv, base >= 6.0 ? 0.0 : base, base);
}

// Hues closer than this to a sector boundary, in sectors, count as on it.
static const float HSV_BOUNDARY = 1e-5;

// darktable's formula for one sector, given its index (0 to 5) and where it starts, in
// sectors.
[Differentiable]
float3 hsv_in_sector(float3 hsv, no_diff float sector, no_diff float base)
{
    float c = hsv.y * hsv.z;
    float m = hsv.z - c;
    float fc = (hsv.x * 6.0 - base) * c;
    float top = c + m;
    float rising = fc + m;
    float falling = top - fc;
    float3 rgb = float3(top, m, falling);
    if (sector == 0.0)
        rgb = float3(top, rising, m);
    else if (sector == 1.0)
        rgb = float3(falling, top, m);
    else if (sector == 2.0)
        rgb = float3(m, top, rising);
    else if (sector == 3.0)
        rgb = float3(m, falling, top);
    else if (sector == 4.0)
        rgb = float3(rising, m, top);
    return rgb;
}

// The two sectors whose slopes the derivatives average, as an index and a start each: on
// a boundary, the sectors either side of it; elsewhere, the hue's own sector twice.
void hsv_sides(float h, out float right, out float right_base, out float left, out float left_base)
{
    float nearest = round(h);
    bool boundary = abs(h - nearest) < HSV_BOUNDARY;
    right_base = boundary ? nearest : floor(h);
    left_base = boundary ? nearest - 1.0 : right_base;
    // Wraps a start of 6 to sector 0 and of -1 to sector 5.
    right = right_base - 6.0 * floor(right_base / 6.0);
    left = left_base - 6.0 * floor(left_base / 6.0);
}

DifferentialPair<float3> hsv_to_rgb_forward(DifferentialPair<float3> hsv)
{
    float right, right_base, left, left_base;
    hsv_sides(hsv.p.x * 6.0, right, right_base, left, left_base);
    let on_right = fwd_diff(hsv_in_sector)(hsv, right, right_base);
    let on_left = fwd_diff(hsv_in_sector)(hsv, left, left_base);
    return diffPair(hsv_to_rgb(hsv.p), 0.5 * (on_right.d + on_left.d));
}

void hsv_to_rgb_backward(inout DifferentialPair<float3> hsv, float3 d_out)
{
    float right, right_base, left, left_base;
    hsv_sides(hsv.p.x * 6.0, right, right_base, left, left_base);
    var on_right = diffPair(hsv.p, float3(0.0));
    var on_left = diffPair(hsv.p, float3(0.0));
    bwd_diff(hsv_in_sector)(on_right, right, right_base, 0.5 * d_out);
    bwd_diff(hsv_in_sector)(on_left, left, left_base, 0.5 * d_out);
    hsv = diffPair(hsv.p, on_right.d + on_left.d);
}

// Illuminants and chromaticities as darktable computes them (src/common/illuminants.h,
// colorspaces_inline_conversions.h and math.h; the CPU side is
// crates/color/src/illuminants.rs and adaptation.rs). The cone spaces' matrices and D50
// constants stay on the CPU, where an operation's setup composes them into its own.

// darktable's NORM_MIN, 2^-16: the smallest norm, luminance or chromaticity y its colour
// maths divides by.
public static const float NORM_MIN = 1.52587890625e-05;

// The pipeline's D50 white: its chromaticity (D50xyY) and its CIE 1976 u'v'.
public static const float2 D50_XY = float2(0.34567, 0.35850);
public static const float2 D50_UV = float2(0.20915914598542354, 0.488075320769787);

// darktable illuminant_xy_to_XYZ: an illuminant's XYZ at Y = 1.
[Differentiable]
public float3 illuminant_xy_to_xyz(float2 xy)
{
    return float3(xy.x / xy.y, 1.0, (1.0 - xy.x - xy.y) / xy.y);
}

// v held at least `floor` from 0, keeping its sign (0 counts as positive).
[Differentiable]
public float away_from_zero(float v, no_diff float floor)
{
    if (abs(v) < floor)
        return v < 0.0 ? -floor : floor;
    return v;
}

[Differentiable]
public float3 away_from_zero(float3 v, no_diff float floor)
{
    return float3(away_from_zero(v.x, floor), away_from_zero(v.y, floor), away_from_zero(v.z, floor));
}

// darktable dt_xyY_to_uvY, chromaticity only: CIE 1976 u'v' from xy. The denominator is
// held at least NORM_MIN of the sum of its terms' sizes from 0, keeping its sign, so u'
// and v' stay within about 1 / NORM_MIN; only chromaticities far outside the spectral
// locus come near, where darktable's division gives infinities.
[Differentiable]
public float2 xy_to_uv(float2 xy)
{
    float terms = 2.0 * abs(xy.x) + 12.0 * abs(xy.y) + 3.0;
    return float2(4.0 * xy.x, 9.0 * xy.y)
        / away_from_zero(-2.0 * xy.x + 12.0 * xy.y + 3.0, NORM_MIN * terms);
}

// darktable dt_uvY_to_xyY, chromaticity only, its denominator guarded likewise.
[Differentiable]
public float2 uv_to_xy(float2 uv)
{
    float terms = 6.0 * abs(uv.x) + 16.0 * abs(uv.y) + 12.0;
    return float2(9.0 * uv.x, 4.0 * uv.y)
        / away_from_zero(6.0 * uv.x - 16.0 * uv.y + 12.0, NORM_MIN * terms);
}

// darktable UCS 22, Aurélien Pierre's uniform colour space of 2022, and CIE xyY, as
// darktable converts to them (src/common/colorspaces_inline_conversions.h; the CPU side
// is crates/color/src/ucs.rs). JCH is lightness J, chroma C and hue H in radians; HSB
// is hue, saturation S = C / B and brightness B; HCB is hue, chroma and brightness. A
// lightness is relative to `l_white`, the L* of the white luminance.

// darktable's D65 white, D65xyY.
public static const float2 D65_XY = float2(0.31271, 0.32902);

// darktable DT_UCS_L_STAR_RANGE, the lightness L* tends to as Y grows, and
// DT_UCS_L_STAR_UPPER_LIMIT, the largest L* converted back to Y.
static const float UCS_L_STAR_RANGE = 2.098883786377;
static const float UCS_L_STAR_UPPER_LIMIT = 2.09885;

// darktable's FLT_MIN, the smallest normal float, which its UCS conversions divide by
// at least.
static const float FLT_MIN = 1.17549435e-38;

// The exponent of a chroma in its brightness, B = J (C^UCS_B_POWER + 1).
static const float UCS_B_POWER = 1.33654221029386;

// d held at least FLT_MIN from 0, keeping its sign, as darktable's UCS divisions are.
[Differentiable]
float away_from_flt_min(float d)
{
    return d >= 0.0 ? max(FLT_MIN, d) : min(-FLT_MIN, d);
}

// darktable dt_D65_XYZ_to_xyY: chromaticity and luminance, negative XYZ clipped to 0
// first; black has D65's chromaticity.
[Differentiable]
public float3 d65_xyz_to_xyy(float3 xyz)
{
    float3 v = max(xyz, float3(0.0));
    float sum = v.x + v.y + v.z;
    float2 xy = D65_XY;
    if (sum > 0.0)
        xy = v.xy / sum;
    return float3(xy.x, xy.y, v.y);
}

// darktable dt_xyY_to_XYZ: black where y is 0.
[Differentiable]
public float3 xyy_to_xyz(float3 xyy)
{
    if (xyy.y == 0.0)
        return float3(0.0);
    return float3(xyy.z * xyy.x / xyy.y, xyy.z, xyy.z * (1.0 - xyy.x - xyy.y) / xyy.y);
}

// darktable Y_to_dt_UCS_L_star: the lightness L* of luminance y, 0 for black.
[Differentiable]
public float y_to_ucs_l_star(float y)
{
    float y_hat = pow0(y, 0.631651345306265);
    return UCS_L_STAR_RANGE * y_hat / (y_hat + 1.12426773749357);
}

// darktable dt_UCS_L_star_to_Y, for L* up to UCS_L_STAR_UPPER_LIMIT.
[Differentiable]
public float ucs_l_star_to_y(float l_star)
{
    return pow0(1.12426773749357 * l_star / (UCS_L_STAR_RANGE - l_star), 1.5831518565279648);
}

// darktable xyY_to_dt_UCS_UV: a chromaticity as U*′ and V*′, whose angle is the hue and
// whose squared length the colourfulness M².
[Differentiable]
public float2 xy_to_ucs_uv(float2 xy)
{
    float3 uvd = float3(-0.783941002840055, 0.745273540913283, 0.318707282433486) * xy.x
        + float3(0.277512987809202, -0.205375866083878, 2.16743692732158) * xy.y
        + float3(0.153836578598858, -0.165478376301988, 0.291320554395942);
    float2 uv = uvd.xy / away_from_flt_min(uvd.z);
    float2 uv_star = float2(1.39656225667, 1.4513954287) * uv
        / (abs(uv) + float2(1.49217352929, 1.52488637914));
    return float2(
        -1.124983854323892 * uv_star.x - 0.980483721769325 * uv_star.y,
        1.86323315098672 * uv_star.x + 1.971853092390862 * uv_star.y);
}

// darktable xyY_to_dt_UCS_JCH, for a D65 xyY whose white has Y 1.
[Differentiable]
public float3 xyy_to_ucs_jch(float3 xyy, no_diff float l_white)
{
    float2 uv = xy_to_ucs_uv(xyy.xy);
    float l_star = y_to_ucs_l_star(xyy.z);
    float m2 = dot(uv, uv);
    float j = l_star / l_white;
    float c = 15.932993652962535 * pow0(l_star, 0.6523997524738018)
        * pow0(m2, 0.6007557017508491) / l_white;
    return float3(j, c, atan2(uv.y, uv.x));
}

// darktable dt_UCS_JCH_to_xyY, L* clipped to the range it converts back from.
[Differentiable]
public float3 ucs_jch_to_xyy(float3 jch, no_diff float l_white)
{
    float l_star = clamp(jch.x * l_white, 0.0, UCS_L_STAR_UPPER_LIMIT);
    float m = 0.0;
    if (l_star != 0.0)
        m = pow0(jch.y * l_white / (15.932993652962535 * pow0(l_star, 0.6523997524738018)),
            0.8322850678616855);
    float2 uv_star_prime = m * float2(cos(jch.z), sin(jch.z));
    float2 uv_star = float2(
        -5.037522385190711 * uv_star_prime.x - 2.504856328185843 * uv_star_prime.y,
        4.760029407436461 * uv_star_prime.x + 2.874012963239247 * uv_star_prime.y);
    float2 uv = -float2(1.49217352929, 1.52488637914) * uv_star
        / (abs(uv_star) - float2(1.39656225667, 1.4513954287));
    float3 xyd = float3(0.167171472114775, -0.150959086409163, 0.940254742367256) * uv.x
        + float3(0.141299802443708, -0.155185060382272, 1.0) * uv.y
        + float3(-0.00801531300850582, -0.00843312433578007, -0.0256325967652889);
    float2 xy = xyd.xy / away_from_flt_min(xyd.z);
    return float3(xy.x, xy.y, ucs_l_star_to_y(l_star));
}

// darktable dt_UCS_JCH_to_HSB.
[Differentiable]
public float3 ucs_jch_to_hsb(float3 jch)
{
    float b = jch.x * (pow0(jch.y, UCS_B_POWER) + 1.0);
    float s = 0.0;
    if (b > 0.0)
        s = jch.y / b;
    return float3(jch.z, s, b);
}

// darktable dt_UCS_HSB_to_JCH.
[Differentiable]
public float3 ucs_hsb_to_jch(float3 hsb)
{
    float c = hsb.y * hsb.z;
    return float3(hsb.z / (pow0(c, UCS_B_POWER) + 1.0), c, hsb.x);
}

// darktable dt_UCS_JCH_to_HCB.
[Differentiable]
public float3 ucs_jch_to_hcb(float3 jch)
{
    return float3(jch.z, jch.y, jch.x * (pow0(jch.y, UCS_B_POWER) + 1.0));
}

// darktable dt_UCS_HCB_to_JCH.
[Differentiable]
public float3 ucs_hcb_to_jch(float3 hcb)
{
    return float3(hcb.z / (pow0(hcb.y, UCS_B_POWER) + 1.0), hcb.y, hcb.x);
}
