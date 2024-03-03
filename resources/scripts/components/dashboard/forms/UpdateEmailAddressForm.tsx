import React from 'react';
import * as Yup from 'yup';
import { pt } from 'yup-locales';
import tw from 'twin.macro';
import { ApplicationStore } from '@/state';
import { httpErrorToHuman } from '@/api/http';
import Field from '@/components/elements/Field';
import { Form, Formik, FormikHelpers } from 'formik';
import { Button } from '@/components/elements/button/index';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import { Actions, State, useStoreActions, useStoreState } from 'easy-peasy';

interface Values {
    email: string;
    password: string;
}

const schema = Yup.object().shape({
    email: Yup.string().email().required(),
    password: Yup.string().required('Deve fornecer a sua senha de conta corrente.'),
});

export default () => {
    const user = useStoreState((state: State<ApplicationStore>) => state.user.data);
    const updateEmail = useStoreActions((state: Actions<ApplicationStore>) => state.user.updateUserEmail);

    const { clearFlashes, addFlash } = useStoreActions((actions: Actions<ApplicationStore>) => actions.flashes);

    Yup.setLocale(pt);

    const submit = (values: Values, { resetForm, setSubmitting }: FormikHelpers<Values>) => {
        clearFlashes('account:email');

        updateEmail({ ...values })
            .then(() =>
                addFlash({
                    type: 'success',
                    key: 'account:email',
                    message: 'O seu E-mail principal foi atualizado.',
                }),
            )
            .catch((error) =>
                addFlash({
                    type: 'danger',
                    key: 'account:email',
                    title: 'Erro',
                    message: httpErrorToHuman(error),
                }),
            )
            .then(() => {
                resetForm();
                setSubmitting(false);
            });
    };

    return (
        <Formik onSubmit={submit} validationSchema={schema} initialValues={{ email: user!.email, password: '' }}>
            {({ isSubmitting, isValid }) => (
                <React.Fragment>
                    <SpinnerOverlay size={'large'} visible={isSubmitting} />
                    <Form css={tw`m-0`}>
                        <Field id={'current_email'} type={'email'} name={'email'} label={'E-mail'} />
                        <div css={tw`mt-6`}>
                            <Field
                                id={'confirm_password'}
                                type={'password'}
                                name={'password'}
                                label={'Confirmar Senha'}
                            />
                        </div>
                        <div css={tw`mt-6`}>
                            <Button disabled={isSubmitting || !isValid}>Atualizar E-mail</Button>
                        </div>
                    </Form>
                </React.Fragment>
            )}
        </Formik>
    );
};
