import React from 'react';
import { useStoreState } from '@/state/hooks';
import getDiscord from '@/api/account/getDiscord';
import { Button } from '@/components/elements/button';

export default () => {
    const discordId = useStoreState((state) => state.user.data!.discordId);

    const link = () => {
        getDiscord().then((data) => {
            window.location.href = data;
        });
    };

    return (
        <>
            {discordId ? (
                <p className={'text-gray-400'}>A sua conta está atualmente ligada ao Discord: {discordId}</p>
            ) : (
                <>
                    <p className={'text-gray-400'}>A sua conta não está ligada ao Discord.</p>
                    <Button.Success className={'mt-4'} onClick={() => link()}>
                        Linkar com o Discord
                    </Button.Success>
                </>
            )}
        </>
    );
};
