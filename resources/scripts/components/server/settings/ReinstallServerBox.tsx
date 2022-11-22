import tw from 'twin.macro';
import { ApplicationStore } from '@/state';
import { httpErrorToHuman } from '@/api/http';
import { ServerContext } from '@/state/server';
import React, { useEffect, useState } from 'react';
import { Actions, useStoreActions } from 'easy-peasy';
import { Dialog } from '@/components/elements/dialog';
import reinstallServer from '@/api/server/reinstallServer';
import { Button } from '@/components/elements/button/index';
import TitledGreyBox from '@/components/elements/TitledGreyBox';

export default () => {
    const uuid = ServerContext.useStoreState((state) => state.server.data!.uuid);
    const [modalVisible, setModalVisible] = useState(false);
    const { addFlash, clearFlashes } = useStoreActions((actions: Actions<ApplicationStore>) => actions.flashes);

    const reinstall = () => {
        clearFlashes('settings');
        reinstallServer(uuid)
            .then(() => {
                addFlash({
                    key: 'settings',
                    type: 'success',
                    message: 'Seu servidor começou o processo de reinstalação.',
                });
            })
            .catch((error) => {
                console.error(error);

                addFlash({ key: 'settings', type: 'error', message: httpErrorToHuman(error) });
            })
            .then(() => setModalVisible(false));
    };

    useEffect(() => {
        clearFlashes();
    }, []);

    return (
        <TitledGreyBox title={'Reinstalar o servidor'} css={tw`relative`}>
            <Dialog.Confirm
                open={modalVisible}
                title={'Confirme a reinstalação do servidor'}
                confirm={'Sim, reinstale o servidor'}
                onClose={() => setModalVisible(false)}
                onConfirmed={reinstall}
            >
                Seu servidor será interrompido e alguns arquivos podem ser excluídos ou modificados durante esse processo, você tem certeza
                Você deseja continuar?
            </Dialog.Confirm>
            <p css={tw`text-sm`}>
            Reinstale para mudar a versão ou para resetar o servidor.&nbsp;
                <strong css={tw`font-medium`}>
                Alguns arquivos podem ser excluídos ou modificados durante esse processo, faça backup de seus dados antes
                    de continuar.
                </strong>
            </p>
            <div css={tw`mt-6 text-right`}>
                <Button.Danger variant={Button.Variants.Secondary} onClick={() => setModalVisible(true)}>
                Reinstalar o servidor
                </Button.Danger>
            </div>
        </TitledGreyBox>
    );
};
