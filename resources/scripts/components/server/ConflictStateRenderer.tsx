import React from 'react';
import { ServerContext } from '@/state/server';
import ScreenBlock from '@/components/elements/ScreenBlock';
import ServerInstallSvg from '@/assets/images/server_installing.svg';
import ServerErrorSvg from '@/assets/images/server_error.svg';
import ServerRestoreSvg from '@/assets/images/server_restore.svg';

export default () => {
    const status = ServerContext.useStoreState((state) => state.server.data?.status || null);
    const isTransferring = ServerContext.useStoreState((state) => state.server.data?.isTransferring || false);
    const isNodeUnderMaintenance = ServerContext.useStoreState(
        (state) => state.server.data?.isNodeUnderMaintenance || false
    );

    return status === 'installing' || status === 'install_failed' ? (
        <ScreenBlock
            title={'Instalador em execução'}
            image={ServerInstallSvg}
            message={'Seu servidor deve estar pronto em breve, tente novamente em alguns minutos.'}
        />
    ) : status === 'suspended' ? (
        <ScreenBlock
            title={'Servidor suspenso'}
            image={ServerErrorSvg}
            message={'Este servidor está suspenso e não pode ser acessado.'}
        />
    ) : isNodeUnderMaintenance ? (
        <ScreenBlock
            title={'Node sob manutenção'}
            image={ServerErrorSvg}
            message={'O Node deste servidor está atualmente sob manutenção.'}
        />
    ) : (
        <ScreenBlock
            title={isTransferring ? 'Transferring' : 'Restoring from Backup'}
            image={ServerRestoreSvg}
            message={
                isTransferring
                    ? 'Seu servidor está sendo transferido para um novo Node, verifique novamente mais tarde.'
                    : 'Seu servidor está sendo restaurado de um backup, verifique em alguns minutos.'
            }
        />
    );
};
