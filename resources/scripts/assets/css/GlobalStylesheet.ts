import tw from 'twin.macro';
import { createGlobalStyle } from 'styled-components';

export default createGlobalStyle`
    body {
        ${tw`bg-neutral-900 font-sans font-medium text-gray-200`};
    }

    *{
        --jexactyl_image_store_one: ${(props) => `url(${props.theme.one})`}
        --jexactyl_image_store_two: ${(props) => `url(${props.theme.two})`}
        --jexactyl_image_store_three: ${(props) => `url(${props.theme.three})`}
    }

    h1, h2, h3, h4, h5, h6 {
        ${tw`tracking-normal`};
    }

    p {
        ${tw`leading-snug`};
    }

    form {
        ${tw`m-0`};
    }

    textarea, select, input, button, button:focus, button:focus-visible {
        ${tw`outline-none`};
    }

    input[type=number]::-webkit-outer-spin-button,
    input[type=number]::-webkit-inner-spin-button {
        -webkit-appearance: none !important;
        margin: 0;
    }

    input[type=number] {
        -moz-appearance: textfield !important;
    }

    /* Scroll Bar Style */
    ::-webkit-scrollbar {
        background: none;
        width: 16px;
        height: 16px;
    }

    ::-webkit-scrollbar-thumb {
        border: solid 0 rgb(0 0 0 / 0%);
        border-right-width: 4px;
        border-left-width: 4px;
        -webkit-border-radius: 9px 4px;
        -webkit-box-shadow: inset 0 0 0 1px hsl(211, 10%, 53%), inset 0 0 0 4px hsl(209deg 18% 30%);
    }

    ::-webkit-scrollbar-track-piece {
        margin: 4px 0;
    }

    ::-webkit-scrollbar-thumb:horizontal {
        border-right-width: 0;
        border-left-width: 0;
        border-top-width: 4px;
        border-bottom-width: 4px;
        -webkit-border-radius: 4px 9px;
    }

    ::-webkit-scrollbar-corner {
        background: transparent;
    }
`;
