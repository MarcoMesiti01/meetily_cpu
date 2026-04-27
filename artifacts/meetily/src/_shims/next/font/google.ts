// Browser stub for `next/font/google`.
// next/font normally generates a CSS class + variable at build time. We can't do
// that statically, so we return a stable variable name and rely on the matching
// <link> tag in index.html (which loads the font from fonts.googleapis.com) to
// actually apply the font.

export interface NextFontOptions {
  subsets?: string[];
  weight?: string | string[];
  style?: string | string[];
  variable?: string;
  display?: string;
  preload?: boolean;
  fallback?: string[];
}

export interface NextFont {
  className: string;
  style: { fontFamily: string };
  variable: string;
}

function makeFont(options: NextFontOptions): NextFont {
  const variable = options.variable ?? "--next-font";
  return {
    className: "",
    style: { fontFamily: `var(${variable})` },
    variable,
  };
}

export const Source_Sans_3 = (options: NextFontOptions): NextFont =>
  makeFont(options);
export const Inter = (options: NextFontOptions): NextFont => makeFont(options);
export const Roboto = (options: NextFontOptions): NextFont => makeFont(options);
export const Open_Sans = (options: NextFontOptions): NextFont =>
  makeFont(options);
export const Lato = (options: NextFontOptions): NextFont => makeFont(options);
export const Poppins = (options: NextFontOptions): NextFont =>
  makeFont(options);
export const Montserrat = (options: NextFontOptions): NextFont =>
  makeFont(options);
export const Raleway = (options: NextFontOptions): NextFont =>
  makeFont(options);
