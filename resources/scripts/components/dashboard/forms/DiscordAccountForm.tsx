import React from 'react';
import { useStoreState } from '@/state/hooks';
import { Button } from '@/components/elements/button';
import { linkDiscord, unlinkDiscord } from '@/api/account/discord';

export default () => {
    const discordId = useStoreState((state) => state.user.data!.discordId);

    const link = () => {
        linkDiscord().then((data) => {
            window.location.href = data;
        });
    };

    const unlink = () => {
        unlinkDiscord().then(() => {
            window.location.href = '/account';
        });
    };

    return (
        <>
            {discordId ? (
                <>
                    <p className={'text-gray-400'}>Sua conta está atualmente vinculada ao Discord: {discordId}</p>
                    <Button.Success className={'mt-4'} onClick={() => unlink()}>
                        Desvincular conta do Discord
                    </Button.Success>
                </>
            ) : (
                <>
                    <p className={'text-gray-400'}>A sua conta não está ligada ao Discord.</p>
                    <Button.Success className={'mt-4'} onClick={() => link()}>
                        Vincular conta do Discord
                    </Button.Success>
                </>
            )}
        </>
    );
};
