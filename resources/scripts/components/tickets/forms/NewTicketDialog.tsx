import React from 'react';
import tw from 'twin.macro';
import { object, string } from 'yup';
import styled from 'styled-components';
import useFlash from '@/plugins/useFlash';
import { httpErrorToHuman } from '@/api/http';
import { createTicket } from '@/api/account/tickets';
import { Field, Form, Formik, FormikHelpers } from 'formik';
import { Button } from '@/components/elements/button/index';
import Input, { Textarea } from '@/components/elements/Input';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import { Dialog, DialogProps } from '@/components/elements/dialog';
import FormikFieldWrapper from '@/components/elements/FormikFieldWrapper';

interface Values {
    title: string;
    description: string;
}

const CustomTextarea = styled(Textarea)`
    ${tw`h-32`}
`;

export default ({ open, onClose }: DialogProps) => {
    const { addError, clearFlashes } = useFlash();

    const onTicketCreated = (id: string) => {
        // @ts-expect-error this is valid
        window.location = '/tickets/view/' + id;
    };

    const submit = (values: Values, { setSubmitting, resetForm }: FormikHelpers<Values>) => {
        clearFlashes('tickets');

        createTicket(values.title, values.description)
            .then((id) => {
                resetForm();
                setSubmitting(false);

                onTicketCreated(id);
            })
            .catch((error) => {
                setSubmitting(false);

                addError({ key: 'tickets', message: httpErrorToHuman(error) });
            });
    };

    return (
        <Dialog
            open={open}
            onClose={onClose}
            title={'Criar um novo ticket'}
            description={'Este ticket será registrado sob sua conta e acessível a todos os administradores do Painel.'}
            preventExternalClose
        >
            <Formik
                onSubmit={submit}
                initialValues={{ title: '', description: '' }}
                validationSchema={object().shape({
                    allowedIps: string(),
                    description: string().required().min(4),
                })}
            >
                {({ isSubmitting }) => (
                    <Form className={'mt-6'}>
                        <SpinnerOverlay visible={isSubmitting} />
                        <FormikFieldWrapper
                            label={'Título'}
                            name={'title'}
                            description={'Um título para este ticket.'}
                            className={'mb-6'}
                        >
                            <Field name={'title'} as={Input} />
                        </FormikFieldWrapper>
                        <FormikFieldWrapper
                            label={'Descrição'}
                            name={'description'}
                            description={
                                'Forneça informações adicionais, imagens e outros conteúdos a fim de nos ajudar a resolver seu problema mais rapidamente.'
                            }
                        >
                            <Field name={'description'} as={CustomTextarea} />
                        </FormikFieldWrapper>
                        <div className={'flex justify-end mt-6'}>
                            <Button type={'submit'}>Criar</Button>
                        </div>
                    </Form>
                )}
            </Formik>
        </Dialog>
    );
};
