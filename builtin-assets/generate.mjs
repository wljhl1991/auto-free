import sharp from 'sharp';
import { writeFileSync, mkdirSync } from 'fs';
import { join } from 'path';

const BASE = join(import.meta.dirname);

// 场景图颜色配置 (1280x720)
const sceneConfigs = {
  visual_novel: [
    { name: 'vn_scene_1', colors: ['#667eea', '#764ba2'], desc: '梦幻渐变' },
    { name: 'vn_scene_2', colors: ['#f093fb', '#f5576c'], desc: '粉色渐变' },
    { name: 'vn_scene_3', colors: ['#4facfe', '#00f2fe'], desc: '天空渐变' },
  ],
  mystery: [
    { name: 'mystery_scene_1', colors: ['#2c3e50', '#4ca1af'], desc: '暗夜渐变' },
    { name: 'mystery_scene_2', colors: ['#1a1a2e', '#16213e'], desc: '深蓝渐变' },
    { name: 'mystery_scene_3', colors: ['#0f0c29', '#302b63'], desc: '紫夜渐变' },
  ],
  horror: [
    { name: 'horror_scene_1', colors: ['#1a1a1a', '#3d0000'], desc: '血色渐变' },
    { name: 'horror_scene_2', colors: ['#0d0d0d', '#1a1a2e'], desc: '深渊渐变' },
    { name: 'horror_scene_3', colors: ['#2d0000', '#4a0000'], desc: '暗红渐变' },
  ],
  rpg: [
    { name: 'rpg_scene_1', colors: ['#56ab2f', '#a8e063'], desc: '森林渐变' },
    { name: 'rpg_scene_2', colors: ['#f7971e', '#ffd200'], desc: '沙漠渐变' },
    { name: 'rpg_scene_3', colors: ['#6a3093', '#a044ff'], desc: '魔法渐变' },
  ],
  simulation: [
    { name: 'sim_scene_1', colors: ['#11998e', '#38ef7d'], desc: '清新渐变' },
    { name: 'sim_scene_2', colors: ['#ee9ca7', '#ffdde1'], desc: '温馨渐变' },
    { name: 'sim_scene_3', colors: ['#2193b0', '#6dd5ed'], desc: '海洋渐变' },
  ],
};

// 头像颜色配置 (256x256)
const portraitConfigs = {
  male: [
    { name: 'male_1', color: '#4a90d9', label: 'M1' },
    { name: 'male_2', color: '#5b9bd5', label: 'M2' },
    { name: 'male_3', color: '#6ba5d7', label: 'M3' },
    { name: 'male_4', color: '#7bafd9', label: 'M4' },
    { name: 'male_5', color: '#8bb9db', label: 'M5' },
  ],
  female: [
    { name: 'female_1', color: '#d94a7a', label: 'F1' },
    { name: 'female_2', color: '#d95b8a', label: 'F2' },
    { name: 'female_3', color: '#d96b9a', label: 'F3' },
    { name: 'female_4', color: '#d97baa', label: 'F4' },
    { name: 'female_5', color: '#d98bba', label: 'F5' },
  ],
};

async function generateGradientImage(width, height, colors, outputPath) {
  // 创建渐变 SVG
  const svg = `<svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" style="stop-color:${colors[0]};stop-opacity:1" />
        <stop offset="100%" style="stop-color:${colors[1]};stop-opacity:1" />
      </linearGradient>
    </defs>
    <rect width="${width}" height="${height}" fill="url(#grad)" />
  </svg>`;

  await sharp(Buffer.from(svg)).png().toFile(outputPath);
  console.log(`Created: ${outputPath}`);
}

async function generatePortraitImage(color, label, outputPath) {
  const svg = `<svg width="256" height="256" xmlns="http://www.w3.org/2000/svg">
    <rect width="256" height="256" fill="${color}" rx="16" />
    <circle cx="128" cy="100" r="40" fill="rgba(255,255,255,0.3)" />
    <ellipse cx="128" cy="200" rx="50" ry="40" fill="rgba(255,255,255,0.2)" />
    <text x="128" y="140" font-family="Arial" font-size="24" fill="white" text-anchor="middle" font-weight="bold">${label}</text>
  </svg>`;

  await sharp(Buffer.from(svg)).png().toFile(outputPath);
  console.log(`Created: ${outputPath}`);
}

// 最小有效 MP3 文件头 (约 140 字节的静音帧)
function createMinimalMp3() {
  // ID3v2 header (10 bytes) + minimal MP3 frame
  const id3Header = Buffer.from([
    0x49, 0x44, 0x33, // "ID3"
    0x03, 0x00,       // Version 2.3
    0x00,             // Flags
    0x00, 0x00, 0x00, 0x00, // Size (0 tags)
  ]);

  // MPEG1 Layer3 128kbps 44100Hz stereo frame header + minimal frame
  const mp3Frame = Buffer.from([
    0xFF, 0xFB, 0x90, 0x00, // Frame header
    ...new Array(413).fill(0), // Frame data (padded)
  ]);

  return Buffer.concat([id3Header, mp3Frame]);
}

async function main() {
  // 生成场景图
  for (const [category, configs] of Object.entries(sceneConfigs)) {
    for (const config of configs) {
      const outputPath = join(BASE, 'images', category, `${config.name}.png`);
      await generateGradientImage(1280, 720, config.colors, outputPath);
    }
  }

  // 生成头像
  for (const [gender, configs] of Object.entries(portraitConfigs)) {
    for (const config of configs) {
      const outputPath = join(BASE, 'portraits', gender, `${config.name}.png`);
      await generatePortraitImage(config.color, config.label, outputPath);
    }
  }

  // 生成音频占位文件
  const mp3Data = createMinimalMp3();
  const audioFiles = [
    'music/calm.mp3', 'music/tense.mp3', 'music/dark.mp3',
    'music/happy.mp3', 'music/battle.mp3',
    'sfx/click.mp3', 'sfx/transition.mp3',
  ];

  for (const file of audioFiles) {
    const outputPath = join(BASE, file);
    writeFileSync(outputPath, mp3Data);
    console.log(`Created: ${outputPath}`);
  }

  console.log('\nAll builtin assets generated successfully!');
}

main().catch(console.error);
