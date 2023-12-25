import { BreakpointFunction, createBreakpoint } from 'styled-components-breakpoint';
import { StorefrontSettings } from './state/storefront';

type Breakpoints = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

export const breakpoint: BreakpointFunction<Breakpoints> = createBreakpoint<Breakpoints>({
    xs: 0,
    sm: 640,
    md: 768,
    lg: 1024,
    xl: 1280,
});

export function Theme(data: StorefrontSettings ) {
    return {
        one: data?.images.one ?? 'https://www.minecraft.net/content/dam/games/minecraft/key-art/MC-Vanilla_Block-Column-Image_Boat-Trip_800x800.jpg',
        two: data?.images.two ?? 'https://www.minecraft.net/content/dam/games/minecraft/key-art/MC-Vanilla_Block-Column-Image_Beach-Cabin_800x800.jpg',
        three: data?.images.three ?? 'https://www.minecraft.net/content/dam/games/minecraft/key-art/MC-Vanilla_Block-Column-Image_Mining_800x800.jpg'
    }
}