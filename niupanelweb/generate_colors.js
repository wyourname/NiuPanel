const chroma = require('chroma-js');

// Function to generate color mix (similar to Element Plus color mixing algorithm for CSS variables)
function mix(color1, color2, weight) {
  return chroma.mix(color1, color2, weight, 'rgb').hex();
}

function generatePalette(primary) {
  const white = '#ffffff';
  const black = '#000000';
  const darkBlack = '#141414'; // Element Plus dark mode background basis

  const colors = { light: {}, dark: {} };

  // Light theme
  colors.light['--el-color-primary'] = primary;
  for (let i = 1; i <= 9; i++) {
    colors.light[`--el-color-primary-light-${i}`] = mix(primary, white, i / 10);
  }
  colors.light['--el-color-primary-dark-2'] = mix(primary, black, 0.2);

  // Dark theme
  colors.dark['--el-color-primary'] = primary;
  const darkMixBase = darkBlack;
  for (let i = 1; i <= 9; i++) {
     colors.dark[`--el-color-primary-light-${i}`] = mix(darkMixBase, primary, 1 - (i / 10));
  }
  colors.dark['--el-color-primary-dark-2'] = mix(primary, white, 0.2);

  return colors;
}

const reqColor = "#3B82F6";
const palette = generatePalette(reqColor);

console.log("=== Light Mode ===");
for(const key in palette.light) {
  console.log(`${key}: ${palette.light[key]};`);
}

console.log("\n=== Dark Mode ===");
for(const key in palette.dark) {
  console.log(`${key}: ${palette.dark[key]};`);
}
