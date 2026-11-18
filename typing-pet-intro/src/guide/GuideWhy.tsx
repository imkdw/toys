import {Easing, Img, Interactive, interpolate, staticFile, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {jua} from '../fonts';
import {Headline, Tag} from '../Text';
import {Panel, Stamp} from './Mac';

// 왜 처음에 막히는지 비개발자에게 설명하는 장면: Apple 검사 도장이 있는 앱과 없는 앱을 나란히 보여 준다.
const GuideWhyInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#ffe1ea', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        잠깐!
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"왜 막히는\n걸까요?"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"위험해서가 아니라\nMac 이 처음 보는 앱이라서예요"}
      </Headline>
      <Pet name="Kurimanju" clip="kuri_haa" frames={48} repeat hold={20} style={{position: 'absolute', left: 110, top: 610, width: 450}} />

      <Panel name="Card checked" style={{left: 770, top: 130, width: 510, height: 440, opacity: interpolate(frame, [34, 46], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [34, 54], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        <div style={{position: 'absolute', left: 185, top: 36, width: 140, height: 140, borderRadius: 34, background: 'linear-gradient(#3fb2ff, #0a6cf0)', color: '#ffffff', fontFamily: jua, fontSize: 96, textAlign: 'center', lineHeight: '140px'}}>A</div>
        <div style={{position: 'absolute', left: 0, right: 0, top: 200, textAlign: 'center', fontFamily: jua, fontSize: 46, color: '#5a3a28'}}>Apple 검사를 받은 앱</div>
        <div style={{position: 'absolute', left: 0, right: 0, top: 268, textAlign: 'center', fontFamily: jua, fontSize: 32, lineHeight: 1.35, color: '#8a6a55'}}>개발자가 해마다 등록비를 내고<br />Apple 에 검사를 맡겨요</div>
      </Panel>
      <Stamp name="Stamp ok" color="#2fa84f" style={{left: 900, top: 512, fontFamily: jua, fontSize: 50, rotate: '-7deg', opacity: interpolate(frame, [62, 66], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), scale: interpolate(frame, [62, 72], [2.2, 1], {easing: Easing.bezier(0.16, 1, 0.3, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        검사 도장 O
      </Stamp>

      <Panel name="Card TypingPet" style={{left: 1330, top: 130, width: 510, height: 440, opacity: interpolate(frame, [84, 96], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [84, 104], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        <Img src={staticFile('guide/app_icon.png')} style={{position: 'absolute', left: 180, top: 30, width: 150}} />
        <div style={{position: 'absolute', left: 0, right: 0, top: 200, textAlign: 'center', fontFamily: jua, fontSize: 46, color: '#5a3a28'}}>TypingPet</div>
        <div style={{position: 'absolute', left: 0, right: 0, top: 268, textAlign: 'center', fontFamily: jua, fontSize: 32, lineHeight: 1.35, color: '#8a6a55'}}>개인이 취미로 만든 앱이라<br />검사를 맡기지 않았어요</div>
      </Panel>
      <Stamp name="Stamp none" color="#e8833a" style={{left: 1470, top: 512, fontFamily: jua, fontSize: 50, rotate: '6deg', opacity: interpolate(frame, [112, 116], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), scale: interpolate(frame, [112, 122], [2.2, 1], {easing: Easing.bezier(0.16, 1, 0.3, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        도장 없음
      </Stamp>

      <Panel name="Mac says" style={{left: 770, top: 655, width: 1070, height: 150, backgroundColor: '#fff3c4', opacity: interpolate(frame, [140, 152], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [140, 160], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        <div style={{position: 'absolute', left: 0, right: 0, top: 22, textAlign: 'center', fontFamily: jua, fontSize: 40, color: '#8a6a55'}}>그래서 Mac 이 한 번 물어보는 거예요</div>
        <div style={{position: 'absolute', left: 0, right: 0, top: 72, textAlign: 'center', fontFamily: jua, fontSize: 52, color: '#5a3a28'}}>“도장 없는 앱인데, 정말 열까요?”</div>
      </Panel>
      <Tag name="Not a virus" style={{left: 775, top: 865, fontSize: 42, backgroundColor: '#d9f7d0', rotate: '-2deg', scale: interpolate(frame, [186, 200], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        바이러스가 발견된 게 아니에요
      </Tag>
      <Tag name="Once" style={{left: 1385, top: 865, fontSize: 42, backgroundColor: '#dff3ff', rotate: '2deg', scale: interpolate(frame, [206, 220], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        처음 한 번만 허락하면 끝
      </Tag>
    </>
  );
};

export const GuideWhy = Interactive.create({Component: GuideWhyInner, layout: 'absolute-fill', schema: {}});
