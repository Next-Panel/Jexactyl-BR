import tw from 'twin.macro';
import { object, string, setLocale } from 'yup';
import { pt } from 'yup-locales';
import * as Icon from 'react-feather';
import React, { useState } from 'react';
import useFlash from '@/plugins/useFlash';
import Can from '@/components/elements/Can';
import { httpErrorToHuman } from '@/api/http';
import { ServerContext } from '@/state/server';
import Modal from '@/components/elements/Modal';
import Field from '@/components/elements/Field';
import Label from '@/components/elements/Label';
import Input from '@/components/elements/Input';
import { Form, Formik, FormikHelpers } from 'formik';
import GreyRowBox from '@/components/elements/GreyRowBox';
import { Button } from '@/components/elements/button/index';
import CopyOnClick from '@/components/elements/CopyOnClick';
import FlashMessageRender from '@/components/FlashMessageRender';
import { ServerDatabase } from '@/api/server/databases/getServerDatabases';
import deleteServerDatabase from '@/api/server/databases/deleteServerDatabase';
import RotatePasswordButton from '@/components/server/databases/RotatePasswordButton';

interface Props {
    database: ServerDatabase;
    className?: string;
}

export default ({ database, className }: Props) => {
    const uuid = ServerContext.useStoreState((state) => state.server.data!.uuid);
    const { addError, clearFlashes } = useFlash();
    const [visible, setVisible] = useState(false);
    const [connectionVisible, setConnectionVisible] = useState(false);

    const appendDatabase = ServerContext.useStoreActions((actions) => actions.databases.appendDatabase);
    const removeDatabase = ServerContext.useStoreActions((actions) => actions.databases.removeDatabase);

    const jdbcConnectionString = `jdbc:mysql://${database.username}${
        database.password ? `:${encodeURIComponent(database.password)}` : ''
    }@${database.connectionString}/${database.name}`;

    setLocale(pt);

    const schema = object().shape({
        confirm: string()
            .required('O nome do Database deve ser fornecido.')
            .oneOf([database.name.split('_', 2)[1], database.name], 'O nome do Database deve ser fornecido.'),
    });

    const submit = (values: { confirm: string }, { setSubmitting }: FormikHelpers<{ confirm: string }>) => {
        clearFlashes();
        deleteServerDatabase(uuid, database.id)
            .then(() => {
                setVisible(false);
                setTimeout(() => removeDatabase(database.id), 150);
            })
            .catch((error) => {
                console.error(error);
                setSubmitting(false);
                addError({
                    key: 'database:delete',
                    message: httpErrorToHuman(error),
                });
            });
    };

    return (
        <>
            <Formik onSubmit={submit} initialValues={{ confirm: '' }} validationSchema={schema} isInitialValid={false}>
                {({ isSubmitting, isValid, resetForm }) => (
                    <Modal
                        visible={visible}
                        dismissable={!isSubmitting}
                        showSpinnerOverlay={isSubmitting}
                        onDismissed={() => {
                            setVisible(false);
                            resetForm();
                        }}
                    >
                        <FlashMessageRender byKey={'database:delete'} css={tw`mb-6`} />
                        <h2 css={tw`text-2xl mb-6`}>Confirme a exclusão do Database</h2>
                        <p css={tw`text-sm`}>
                            A exclusão de um Database é uma ação permanente, não pode ser desfeita. Isso vai excluir
                            permanentemente o Database: <strong>{database.name}</strong> e todos os dados associados.
                        </p>
                        <Form css={tw`m-0 mt-6`}>
                            <Field
                                type={'text'}
                                id={'confirm_name'}
                                name={'confirm'}
                                label={'Confirme o nome do Database'}
                                description={'Digite o nome do Database para confirmar a exclusão.'}
                            />
                            <div css={tw`mt-6 text-right`}>
                                <Button
                                    type={'button'}
                                    variant={Button.Variants.Secondary}
                                    css={tw`mr-2`}
                                    onClick={() => setVisible(false)}
                                >
                                    Cancelar
                                </Button>
                                <Button type={'submit'} color={'red'} disabled={!isValid}>
                                    Excluir Database
                                </Button>
                            </div>
                        </Form>
                    </Modal>
                )}
            </Formik>
            <Modal visible={connectionVisible} onDismissed={() => setConnectionVisible(false)}>
                <FlashMessageRender byKey={'database-connection-modal'} css={tw`mb-6`} />
                <h3 css={tw`mb-6 text-2xl`}>Detalhes da conexão do Database</h3>
                <div>
                    <Label>Endpoint</Label>
                    <CopyOnClick text={database.connectionString}>
                        <Input type={'text'} readOnly value={database.connectionString} />
                    </CopyOnClick>
                </div>
                <div css={tw`mt-6`}>
                    <Label>Conexões de</Label>
                    <Input type={'text'} readOnly value={database.allowConnectionsFrom} />
                </div>
                <div css={tw`mt-6`}>
                    <Label>Username</Label>
                    <CopyOnClick text={database.username}>
                        <Input type={'text'} readOnly value={database.username} />
                    </CopyOnClick>
                </div>
                <Can action={'database.view_password'}>
                    <div css={tw`mt-6`}>
                        <Label>Senha</Label>
                        <CopyOnClick text={database.password}>
                            <Input type={'text'} readOnly value={database.password} />
                        </CopyOnClick>
                    </div>
                </Can>
                <div css={tw`mt-6`}>
                    <Label>String de conexão JDBC</Label>
                    <CopyOnClick text={jdbcConnectionString}>
                        <Input type={'text'} readOnly value={jdbcConnectionString} />
                    </CopyOnClick>
                </div>
                <div css={tw`mt-6 text-right`}>
                    <Can action={'database.update'}>
                        <RotatePasswordButton databaseId={database.id} onUpdate={appendDatabase} />
                    </Can>
                    <Button variant={Button.Variants.Secondary} onClick={() => setConnectionVisible(false)}>
                        Perto
                    </Button>
                </div>
            </Modal>
            <GreyRowBox $hoverable={false} className={className} css={tw`mb-2`}>
                <div css={tw`hidden md:block`}>
                    <Icon.Database />
                </div>
                <div css={tw`flex-1 ml-4`}>
                    <CopyOnClick text={database.name}>
                        <p css={tw`text-lg`}>{database.name}</p>
                    </CopyOnClick>
                </div>
                <div css={tw`ml-8 text-center hidden md:block`}>
                    <CopyOnClick text={database.connectionString}>
                        <p css={tw`text-sm`}>{database.connectionString}</p>
                    </CopyOnClick>
                    <p css={tw`mt-1 text-2xs text-neutral-500 uppercase select-none`}>Endpoint</p>
                </div>
                <div css={tw`ml-8 text-center hidden md:block`}>
                    <p css={tw`text-sm`}>{database.allowConnectionsFrom}</p>
                    <p css={tw`mt-1 text-2xs text-neutral-500 uppercase select-none`}>Conexões de</p>
                </div>
                <div css={tw`ml-8 text-center hidden md:block`}>
                    <CopyOnClick text={database.username}>
                        <p css={tw`text-sm`}>{database.username}</p>
                    </CopyOnClick>
                    <p css={tw`mt-1 text-2xs text-neutral-500 uppercase select-none`}>Username</p>
                </div>
                <div css={tw`ml-8`}>
                    <Button
                        variant={Button.Variants.Secondary}
                        css={tw`mr-2`}
                        onClick={() => setConnectionVisible(true)}
                    >
                        <Icon.Eye />
                    </Button>
                    <Can action={'database.delete'}>
                        <Button.Danger variant={Button.Variants.Secondary} onClick={() => setVisible(true)}>
                            <Icon.Trash />
                        </Button.Danger>
                    </Can>
                </div>
            </GreyRowBox>
        </>
    );
};
